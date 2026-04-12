# Lycan

Lightweight PWA manager for Linux. Turn any website into a desktop application with minimal resource overhead.

Lycan uses WebKitGTK to run web apps in standalone windows, generates `.desktop` files for menu integration, and includes a built-in ad/tracker blocker to keep things lean.

## Features

- **TUI interface** for managing your PWA collection — add, edit, delete, search
- **Automatic favicon fetching** from URLs
- **`.desktop` file generation** for rofi, dmenu, application menus
- **Ad/tracker blocking** — blocks 35+ common ad and tracking domains at the network level
- **NVIDIA detection** — automatically applies software rendering workarounds
- **X11 and Wayland** support

## Dependencies

- `gtk3`
- `webkit2gtk`
- `glib2`
- `openssl`

## Installation

### Arch Linux (AUR)

```
yay -S lycan-git
```

### Build from source

```
git clone https://github.com/tutkuofnight/lycan.git
cd lycan
cargo build --release
cp target/release/lycan ~/.local/bin/
```

## Usage

Launch the TUI to manage your PWAs:

```
lycan
```

Open an existing PWA directly:

```
lycan open <app-id>
```

### TUI keybindings

| Key | Action |
|-----|--------|
| `a` | Add new PWA |
| `e` | Edit selected PWA |
| `o` / `Enter` | Open selected PWA |
| `d` | Delete selected PWA |
| `/` | Search / filter |
| `j` / `k` | Navigate up / down |
| `q` | Quit |

## How it works

When you add a PWA, Lycan:

1. Fetches the favicon from the URL and saves it locally
2. Creates a config file in `~/.local/share/lycan/apps/<app-id>/`
3. Generates a `.desktop` file in `~/.local/share/applications/`

When you open a PWA, it launches a WebKitGTK window with the configured URL, using a Chrome user agent for maximum site compatibility.

## Configuration

All data is stored under `~/.local/share/lycan/`. Each PWA gets its own directory:

```
~/.local/share/lycan/apps/
├── whatsapp/
│   ├── config.json
│   └── icon.png
├── youtube/
│   ├── config.json
│   └── icon.png
```

## License

MIT
