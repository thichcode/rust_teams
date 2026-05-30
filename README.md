<div align="center">

# 🦀 Rust Teams

### Lightweight Microsoft Teams Desktop Client

[![CI](https://github.com/thichcode/rust_teams/actions/workflows/ci.yml/badge.svg)](https://github.com/thichcode/rust_teams/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.77+-orange.svg)](https://rust-lang.org)

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

### Download

Download the latest release from [Releases](https://github.com/thichcode/rust_teams/releases) or build from source.

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
- **Rust 1.77+** (for building from source)

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

### Privacy & Control

- 🛡️ **Ad Blocking** — Built-in blocked domains list
- 🕵️ **Tracker Blocking** — Block Google Analytics, DoubleClick
- 🔧 **DevTools** — Press F12 to inspect (for development)
- 📝 **Configurable** — Full JSON config file

---

## 🛠️ Configuration

Config file location: `%APPDATA%\thuong\rust_teams\config\config.json`

```json
{
  "profiles": [
    {
      "id": "default",
      "name": "Microsoft Teams",
      "teams_url": "https://teams.microsoft.com",
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
      "teams_url": "https://teams.microsoft.com",
      "is_default": true
    },
    {
      "id": "personal",
      "name": "Personal Teams",
      "teams_url": "https://teams.microsoft.com",
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
├── config/
│   ├── default_config.json # Default configuration
│   └── blocked_domains.json # Ad/tracker blocklist
├── src/
│   ├── main.rs             # Entry point + WebView setup
│   ├── app.rs              # Config types + Memory optimization
│   ├── config.rs           # Config management
│   ├── error.rs            # Error types
│   └── ui/
│       └── mod.rs          # Window settings
├── Cargo.toml
├── compare.md              # vs MS Teams comparison
├── MEMORY_OPTIMIZED.md     # Memory optimization docs
└── LICENSE
```

---

## 🧠 Memory Optimization

Rust Teams is designed to be lightweight:

| Optimization | RAM Savings |
|---|---|
| Cache limiting (10MB) | ~50-100 MB |
| GPU disabled | ~30-50 MB |
| Animations off | ~10-20 MB |
| Context menus off | ~5-10 MB |
| **Total** | **~95-180 MB** |

See [MEMORY_OPTIMIZED.md](MEMORY_OPTIMIZED.md) for detailed benchmarks.

---

## 🏗️ Tech Stack

| Component | Technology |
|---|---|
| **Language** | Rust |
| **Window Manager** | [tao](https://github.com/tauri-apps/tao) |
| **WebView** | [wry](https://github.com/tauri-apps/wry) + WebView2 |
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