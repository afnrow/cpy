# Cpy 
A simple, high-performance clipboard manager for Wayland written in Rust.

## Features
- **Lightweight**: Minimal CPU and RAM usage.
- **Deduplication**: Doesn't save the same thing twice.
- **Clean**: Automatically strips HTML for the picker view.
- **Images**: Supports `image/png` (Rendering requires `rofi-wayland`).

## Installation
### Prerequisites
- `wl-clipboard`
- `rofi-wayland` (Note: `wofi` works if you edit `src/main.rs`, but images won't render).
- `rust` / `cargo`

### Building the Project
```bash
git clone [https://github.com/afnrow/cpy.git]
cd cpy
cargo install --path .
```
then add a system file like this
```bash
  mkdir -p ~/.config/systemd/user
  nano ~/.config/systemd/user/cpy.service
```
then paste this 
```
[Unit]
Description=cpy Clipboard Daemon
After=graphical-session.target
[Service]
ExecStart=%h/.cargo/bin/cpy daemon
Restart=always
[Install]
WantedBy=graphical-session.target
```
# Rofi/Wayland
You can get all of your copies by rendering it via `~/.cargo/bin/cpy pick` it will open a rofi menu (you can change it to wofi from the main.rs file)
# License
This software is licensed under the GPLv3 License , Check the `License` File for more information
