# termpp

A cross-platform terminal multiplexer built with Rust and [iced](https://github.com/iced-rs/iced).

Inspired by tools like cmux (macOS-only), termpp aims to bring a native GUI multiplexer to Windows, macOS, and Linux.

> **Status:** early development — functional but rough edges remain.

---

## Features

- **Workspaces / Tabs / Panes** — organize sessions in a three-level hierarchy
- **Split panes** — horizontal and vertical splits, resizable
- **Scrollback buffer** — scroll through terminal history with the mouse wheel
- **Text selection** — click-drag to select, copy on release, paste on right-click
- **Per-pane zoom** — Ctrl+Scroll to zoom, Ctrl+0 to reset
- **Configurable** — TOML config file for font, shell, keybindings, and more
- **Help overlay** — F1 shows all shortcuts at any time

---

## Build

### Windows

```sh
cargo build --release
```

### Linux

Install system dependencies first (Debian/Ubuntu/Pop!_OS):

```sh
sudo apt install \
  build-essential pkg-config \
  libxkbcommon-dev \
  libwayland-dev \
  libegl-dev \
  libvulkan-dev \
  mesa-vulkan-drivers \
  libfontconfig-dev
```

Then:

```sh
cargo build --release
```

> **Note:** termpp is a GUI application and requires a display to run. It cannot be launched over a headless SSH session — compile remotely, test locally.

The binary ends up at `target/release/termpp` (or `termpp.exe` on Windows).

**Requirements:** Rust stable, a working PTY backend (included via `portable-pty`).

---

## Configuration

Config file location:

- **Windows:** `%APPDATA%\termpp\config.toml`
- **macOS/Linux:** `~/.config/termpp/config.toml`

If the file does not exist, defaults are used.

### Options

```toml
font_name            = "Cascadia Mono"   # any monospace font installed on the system
font_size            = 14
theme                = "dark"            # only "dark" supported for now
shell                = "pwsh.exe"        # default: pwsh.exe on Windows, $SHELL elsewhere
notification_timeout = 2                 # seconds
auto_close_on_exit   = false

[keybindings]
split_horizontal = "Ctrl + Shift + H"
split_vertical   = "Ctrl + Shift + V"
pane_next        = "Ctrl + Tab"
pane_prev        = "Ctrl + Shift + Tab"
close_pane       = "Ctrl + Shift + Q"
rename_pane      = "Ctrl + Shift + R"
tab_next         = "Ctrl + PageDown"
tab_prev         = "Ctrl + PageUp"
tab_new          = "Ctrl + Shift + T"
workspace_next   = "Ctrl + Shift + PageDown"
workspace_prev   = "Ctrl + Shift + PageUp"
workspace_new    = "Ctrl + Shift + W"
```

---

## Keyboard shortcuts

| Action              | Default              |
|---------------------|----------------------|
| Split horizontal    | Ctrl + Shift + H     |
| Split vertical      | Ctrl + Shift + V     |
| Next pane           | Ctrl + Tab           |
| Previous pane       | Ctrl + Shift + Tab   |
| Close pane          | Ctrl + Shift + Q     |
| Rename pane         | Ctrl + Shift + R     |
| Next tab            | Ctrl + PageDown      |
| Previous tab        | Ctrl + PageUp        |
| New tab             | Ctrl + Shift + T     |
| Next workspace      | Ctrl + Shift + PageDown |
| Previous workspace  | Ctrl + Shift + PageUp   |
| New workspace       | Ctrl + Shift + W     |
| Help overlay        | F1                   |

## Mouse

| Action              | Gesture                  |
|---------------------|--------------------------|
| Select text         | Left click + drag        |
| Select word         | Double-click             |
| Copy selection      | Release click            |
| Paste               | Right-click              |
| Scroll history      | Scroll wheel             |
| Zoom in/out         | Ctrl + Scroll            |
| Reset zoom          | Ctrl + 0                 |

---

## Roadmap / Known gaps

- [ ] **i18n** — UI strings are currently in French; needs translation to English (and ideally proper i18n support)
- [ ] **Cross-platform testing** — macOS and Linux are untested; PTY layer should work but not validated
- [ ] **Themes** — only a dark theme exists; light theme and custom colors not yet supported
- [ ] **Packaging** — no installer or pre-built binaries yet
- [ ] **Screenshots** — none yet
