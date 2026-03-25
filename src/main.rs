use tokio::process::Command as AsyncCommand;
use tokio::io::{AsyncBufReadExt, BufReader};
use std::process::{Command as SyncCommand, Stdio};
use std::io::Write;
use std::fs;
use std::path::{Path, PathBuf};
use chrono::Local;
use walkdir::WalkDir;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const STORAGE_DIR: &str = "/tmp/cpy";
const MAX_ITEMS: usize = 100;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("daemon");
    match mode {
        "daemon" => run_daemon().await?,
        "pick" => show_selector()?,
        "clear" => {
            let _ = fs::remove_dir_all(STORAGE_DIR);
            let _ = fs::create_dir_all(STORAGE_DIR);
            println!("History cleared.");
        },
        _ => println!("Usage: cpy [daemon|pick|clear]"),
    }
    Ok(())
}

async fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    let _ = fs::create_dir_all(STORAGE_DIR);
    let mut last_hash: u64 = 0;
    let mut child = AsyncCommand::new("wl-paste")
        .arg("--watch")
        .arg("echo")
        .arg("changed")
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let mut reader = BufReader::new(stdout).lines();
    while let Some(_line) = reader.next_line().await? {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        if let Ok(data) = capture_plain_text().await {
            let mut hasher = DefaultHasher::new();
            data.hash(&mut hasher);
            let current_hash = hasher.finish();
            if current_hash != last_hash {
                save_to_disk(&data, "txt").await?;
                last_hash = current_hash;
                let _ = prune_history();
            }
        }
    }
    Ok(())
}

async fn capture_plain_text() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let content = SyncCommand::new("wl-paste")
        .arg("--type").arg("text/plain")
        .output()?;
    if content.stdout.is_empty() {
        let img = SyncCommand::new("wl-paste").arg("--type").arg("image/png").output()?;
        if !img.stdout.is_empty() { return Ok(img.stdout); }
        return Err("Empty".into());
    }
    Ok(content.stdout)
}

async fn save_to_disk(data: &[u8], ext: &str) -> tokio::io::Result<()> {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S_%f").to_string();
    let filename = format!("{}/{}.{}", STORAGE_DIR, timestamp, ext);
    fs::write(filename, data)?;
    Ok(())
}

fn prune_history() -> Result<(), Box<dyn std::error::Error>> {
    let mut entries: Vec<_> = WalkDir::new(STORAGE_DIR).into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()).collect();
    if entries.len() > MAX_ITEMS {
        entries.sort_by_key(|e| e.metadata().unwrap().modified().unwrap());
        for i in 0..(entries.len() - MAX_ITEMS) { let _ = fs::remove_file(entries[i].path()); }
    }
    Ok(())
}

fn show_selector() -> Result<(), Box<dyn std::error::Error>> {
    let mut items = Vec::new();
    let mut paths = Vec::new();
    items.push("🗑️  Clear All History".to_string());
    paths.push(PathBuf::from("INTERNAL_CLEAR"));
    let mut entries: Vec<_> = WalkDir::new(STORAGE_DIR)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();
    entries.sort_by_key(|a| std::cmp::Reverse(a.metadata().unwrap().modified().unwrap()));
    for entry in entries {
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let display = if ext == "txt" {
            let text = fs::read_to_string(path).unwrap_or_default();
            text.chars().take(100).collect::<String>().replace('\n', " ").trim().to_string()
        } else {
            format!("🖼️  [IMAGE] - {}", path.file_name().unwrap().to_string_lossy())
        };
        if !display.is_empty() {
            items.push(display);
            paths.push(path.to_path_buf());
        }
    }
    let mut child = SyncCommand::new("rofi")
        .arg("-dmenu")
        .arg("-format").arg("i")
        .arg("-p").arg("Clipboard")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(items.join("\n").as_bytes())?;
    drop(stdin);
    let output = child.wait_with_output()?;
    let selection = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if let Ok(index) = selection.parse::<usize>() {
        if index == 0 {
            let _ = fs::remove_dir_all(STORAGE_DIR);
            let _ = fs::create_dir_all(STORAGE_DIR);
            let _ = SyncCommand::new("notify-send").arg("Clipboard").arg("History Cleared").spawn();
            return Ok(());
        }
        if let Some(selected_path) = paths.get(index) {
            let ext = selected_path.extension().and_then(|s| s.to_str()).unwrap_or("txt");
            let mime = if ext == "png" { "image/png" } else { "text/plain" };
            let file = fs::File::open(selected_path)?;
            SyncCommand::new("wl-copy")
                .arg("--type").arg(mime)
                .stdin(Stdio::from(file))
                .spawn()?;
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }
    Ok(())
}
