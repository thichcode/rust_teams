<div align="center">

# 🦀 Rust Teams

### Lightweight Microsoft Teams Desktop Client

[![CI](https://github.com/thichcode/rust_teams/actions/workflows/ci.yml/badge.svg)](https://github.com/thichcode/rust_teams/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.96+-orange.svg)](https://rust-lang.org)

</div>

---

<p align="center">
  <b>A blazingly fast, memory-efficient Microsoft Teams client built with Rust + WebView2</b>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Size-~10MB-green?style=flat-square" alt="Size">
  <img src="https://img.shields.io/badge/RAM-~80MB-blue?style=flat-square" alt="RAM">
  <img src="https://img.shields.io/badge/Startup-~1.5s-yellow?style=flat-square" alt="Startup">
  <img src="https://img.shields.io/badge/Engine-WebView2-purple?style=flat-square" alt="Engine">
</p>

---

## 📊 Why Rust Teams?

| | Rust Teams | MS Teams (Electron) |
|---|---|---|
| 📦 **Size** | ~10 MB | ~300 MB |
| 💾 **RAM** | ~80-150 MB | ~400-800 MB |
| ⚡ **Startup** | ~1.5s | ~7s |
| 🔋 **CPU Idle** | ~0.5% | ~3% |
| 🔒 **Privacy** | Block tracking/ads | No control |
| 🎨 **Customizable** | Full config control | Limited |

> **Save 60%+ RAM** by using system WebView2 instead of bundling Chromium

---

## 🚀 Quick Start

### Windows

Download the latest `.exe` or `.zip` from [Releases](https://github.com/thichcode/rust_teams/releases).

### Linux (Ubuntu 22.04+)

**Option 1 — Install via .deb (recommended):**

```bash
# Download the latest .deb from Releases
wget https://github.com/thichcode/rust_teams/releases/download/v0.9.64/rust-teams_0.9.64_amd64.deb

# Install
sudo dpkg -i rust-teams_0.9.64_amd64.deb
sudo apt-get install -f   # install missing dependencies if any
```

After installation, launch from app menu or run `rust_teams`.

> **Linux rendering backend:** The app auto-detects a Chromium-based browser
> (Google Chrome / Chromium / Edge / Brave) and launches Teams in a clean
> app-mode window (`--app=`). This gives full Chromium rendering support —
> required on Ubuntu 22.04 where the bundled WebKitGTK (2.40) is too old for
> the modern Teams web app. If no Chromium browser is found, it falls back to
> the embedded WebKitGTK webview.
>
> Force a backend with `--backend auto|webkit|chromium`, or set
> `"linux_backend": "webkit"` in `~/.config/rust-teams/config.json`.
> Use `--backend webkit` if you prefer the embedded window.

**Option 2 — Binary archive:**

```bash
wget https://github.com/thichcode/rust_teams/releases/download/v0.9.64/rust_teams-linux-x64.tar.gz
tar xzf rust_teams-linux-x64.tar.gz
./rust_teams-linux-x64
```

### Build from Source

```bash
# Clone the repository
git clone https://github.com/thichcode/rust_teams.git
cd rust_teams

# Build (release mode)
cargo build --release

# Run
cargo run --release
```

### Requirements

- **Windows 10/11** with WebView2 Runtime (pre-installed on Windows 11)
- **Linux (Ubuntu 22.04+)** with WebKitGTK 4.1 (`libwebkit2gtk-4.1-dev`)
- **Rust 1.96+** (for building from source)

---

## 📸 Screenshots

<p align="center">
  <i>Add your own screenshots here! See comments in README.md for instructions.</i>
</p>

---

## ⚙️ Features

### Core

- 🌐 **Teams Web App** — Full Microsoft Teams experience via WebView2
- 💬 **Chat & Messaging** — Send and receive messages
- 📹 **Video Calls** — Join and host video meetings
- 🖥️ **Screen Sharing** — Share your screen in meetings
- 📁 **File Sharing** — Share files through Teams

### Memory Optimized

- 🧠 **Smart Cache** — 10MB cache limit (vs 250MB default)
- 🚫 **GPU Disabled** — Saves ~30-50MB RAM
- 🎭 **No Animations** — Smooth scrolling disabled for performance
- 📉 **Low CPU** — Minimal background resource usage

### Local LLM Mode

- 🖥 **Offline STT** — whisper.cpp auto-downloaded on first run
- 🤖 **Local Translation & Suggestions** — Ollama with any installed model
- 🔄 **Hybrid Providers** — Mix local + cloud per pipeline role
- ⚙️ **3-Step Wizard** — Pick models for STT, translator, suggester
- ✅ **Readiness Check** — Verifies Ollama + Whisper availability

### Realtime Translate

- 🟢🔴 **Manual On/Off Toggle** — Start/stop translate anytime, independent of meetings
- 🎤 **WASAPI Loopback** — Captures system audio from any app (Teams, Zoom, Discord, etc.)
- 🔄 **STT → Translate → Suggestions** — Real-time pipeline with local or cloud providers
- ⚙️ **In-Panel Configure** — Set API keys directly from the panel
- 🖥 **Local Mode** — whisper.cpp + Ollama for fully offline translation
- 📥 **Auto-Download** — Whisper model downloaded automatically on first use

### Privacy & Control

- 🛡️ **Ad Blocking** — Built-in blocked domains list
- 🕵️ **Tracker Blocking** — Block Google Analytics, DoubleClick
- 🔧 **DevTools** — Press F12 to inspect (for development)
- 📝 **Configurable** — Full JSON config file

---

## 🛠️ Configuration

The configuration system supports multiple profiles, memory optimization, and window settings.

| Setting | Location |
|---------|----------|
| **User config** | `%APPDATA%\rust-teams\app\config\config.json` |
| **Blocked domains** | `config/blocked_domains.json` |
| **Default template** | `config/default_config.json` |

> Config file is auto-created on first launch with optimized defaults.

**Default configuration:**

```json
{
  "profiles": [
    {
      "id": "default",
      "name": "Microsoft Teams",
      "teams_url": "https://teams.microsoft.com/v2/",
      "is_default": true
    }
  ],
  "current_profile_id": "default",
  "window_settings": {
    "width": 1200,
    "height": 800,
    "x": null,
    "y": null,
    "maximized": false
  },
  "memory_optimization": {
    "enabled": true,
    "max_cache_size_mb": 10,
    "disable_gpu": true,
    "disable_animations": true,
    "idle_timeout_secs": 300
  }
}
```

### Multiple Profiles

```json
{
  "profiles": [
    {
      "id": "work",
      "name": "Work Teams",
      "teams_url": "https://teams.microsoft.com/v2/",
      "is_default": true
    },
    {
      "id": "personal",
      "name": "Personal Teams",
      "teams_url": "https://teams.microsoft.com/v2/",
      "is_default": false
    }
  ],
  "current_profile_id": "work"
}
```

---

## 📁 Project Structure

```
rust_teams/
├── .github/
│   └── workflows/
│       ├── ci.yml          # CI pipeline
│       └── build.yml       # Build & Release
├── linux/
│   └── rust_teams.desktop  # Linux desktop entry (.deb)
├── config/
│   ├── default_config.json # Default configuration
│   └── blocked_domains.json # Ad/tracker blocklist
├── src/
│   ├── main.rs             # Entry point + WebView setup + IPC handlers
│   ├── app.rs              # Config types + Memory optimization
│   ├── config.rs           # Config management (save/load API keys, presets)
│   ├── error.rs            # Error types
│   ├── memory.rs           # Chromium flags profiles
│   ├── updater.rs          # Auto-update
│   ├── meeting/
│   │   ├── mod.rs          # Meeting detection + LocalPreset export
│   │   ├── local_check.rs  # Local readiness + Ollama client + wizard options
│   │   ├── realtime_config.rs # RealtimeTranslateConfig + LocalPreset
│   │   ├── pipeline.rs     # STT → Translate → Suggest pipeline
│   │   ├── audio.rs        # Audio capture (cpal + WASAPI loopback)
│   │   ├── loopback.rs     # WASAPI loopback capture
│   │   ├── whisper_download.rs # Auto-download whisper.cpp binary + model
│   │   ├── translate.rs    # Translation providers
│   │   └── notes.rs        # Meeting notes
│   └── ui/
│       ├── mod.rs          # Window settings + IPC registration
│       ├── badge.rs        # Unread badge
│       ├── auto_read.rs    # Auto-read v2
│       └── realtime_panel.rs # Panel + configure modal + local wizard
├── docs/
│   ├── MEMORY.md           # Memory optimization flags reference
│   ├── CODE_SIGNING.md     # Certificate setup guide
│   └── LOCAL_LLM.md        # Local LLM mode user guide
├── Cargo.toml
├── CHANGELOG.md
├── README.md
└── LICENSE
```

---

## 🧠 Memory Optimization

Rust Teams saves **~250MB RAM** vs default WebView2 via 3 profiles:

| Profile | RAM Saved | CLI Flag | Trade-off |
|---|---|---|---|
| Safe | ~70MB | `--memory-profile safe` | None — keeps Spectre protection + cache |
| **Balanced** (default) | ~250MB | `--memory-profile balanced` | Spectre mitigation OFF, BFCache OFF |
| Aggressive | ~350MB | `--memory-profile aggressive` | Caps renderer at 2 + V8 heap at 512MB |
| Off | 0 | `--memory-profile off` | WebView2 defaults |

Also includes JS-level optimizations:
- **Preconnect hints** for Teams CDN
- **Visibility-pause** when tab is hidden
- **Idle GC hint** after 30s inactivity
- **Content-visibility** on off-screen chat lists

See [docs/MEMORY.md](docs/MEMORY.md) for full reference.

---

## 🏗️ Tech Stack

| Component | Technology |
|---|---|
| **Language** | Rust |
| **Window Manager** | [tao](https://github.com/tauri-apps/tao) |
| **WebView** | [wry](https://github.com/tauri-apps/wry) + WebView2 |
| **Audio Capture** | cpal + WASAPI loopback |
| **Local STT** | whisper.cpp (subprocess) |
| **Local LLM** | [Ollama](https://ollama.com) |
| **Config** | serde + serde_json |
| **Error Handling** | anyhow + thiserror |
| **Logging** | env_logger |

---

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

<div align="center">

**Made with 🦀 Rust**

*If you find this project useful, consider giving it a ⭐ star!*

</div>