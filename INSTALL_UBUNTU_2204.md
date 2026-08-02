# Rust Teams — Ubuntu 22.04 Desktop Install Guide

Binary targets **x86_64 Linux** with WebKitGTK 4.1. Runs on Ubuntu 22.04+ Desktop (GNOME / Wayland / X11).

---

## 1. Install via .deb (recommended)

```bash
# Download the latest .deb from Releases
VERSION="v0.9.66"   # or latest from https://github.com/thichcode/rust_teams/releases
wget "https://github.com/thichcode/rust_teams/releases/download/${VERSION}/rust-teams_${VERSION#v}_amd64.deb"

# Install
sudo dpkg -i rust-teams_${VERSION#v}_amd64.deb
sudo apt-get install -f   # install missing dependencies if any
```

**What the .deb installs:**

| Path | Purpose |
|------|---------|
| `/usr/bin/rust_teams` | Binary |
| `/usr/share/applications/rust_teams.desktop` | App menu entry + `msteams://` URL handler |
| `/usr/share/icons/hicolor/256x256/apps/rust_teams.png` | App icon |

After install, search **"R Teams"** in the app launcher or run `rust_teams`.

---

## 2. Manual install (tar.gz)

If you prefer not to use .deb:

```bash
VERSION="v0.9.66"   # or latest from https://github.com/thichcode/rust_teams/releases
cd /tmp
wget "https://github.com/thichcode/rust_teams/releases/download/${VERSION}/rust_teams-linux-x64.tar.gz"
wget "https://github.com/thichcode/rust_teams/releases/download/${VERSION}/rust_teams-linux-x64.tar.gz.sha256"
sha256sum -c rust_teams-linux-x64.tar.gz.sha256 && echo "checksum OK"
tar xzf rust_teams-linux-x64.tar.gz
chmod +x rust_teams-linux-x64
sudo mv rust_teams-linux-x64 /usr/local/bin/rust-teams
```

---

## 3. Build from source

```bash
sudo apt update && sudo apt install -y \
  build-essential pkg-config libssl-dev \
  libgtk-3-dev libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev \
  libsoup-3.0-dev libappindicator3-dev librsvg2-dev \
  libxkbcommon-dev libfontconfig1-dev libdbus-1-dev \
  libasound2-dev libpulse-dev libx11-dev libxcomposite-dev \
  libxdamage-dev libxfixes-dev libxrandr-dev libxi-dev \
  libxrender-dev libxtst-dev libatk1.0-dev libcairo2-dev \
  libcups2-dev libdrm-dev libgbm-dev libgdk-pixbuf-2.0-dev \
  libglib2.0-dev libnspr4-dev libnss3-dev libpango1.0-dev \
  libxcb1-dev libxshmfence-dev libsecret-1-dev patchelf

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup default stable

git clone https://github.com/thichcode/rust_teams.git
cd rust_teams
cargo build --release

# binary at target/release/rust_teams
cp target/release/rust_teams ~/.local/bin/rust-teams
```

---

## 3.5 Install a Chromium-based browser (optional)

The app **auto-launches Teams in a Chromium app-mode window** for full rendering (WebKitGTK 2.40 is too old for the modern Teams SPA). If no browser is found, the app **auto-downloads a portable Chrome** on first run. Installing one system-wide is preferred (faster startup, no download).

**Google Chrome (recommended — best Teams support):**
```bash
wget -q -O /tmp/google-chrome.deb https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb
sudo apt install -y /tmp/google-chrome.deb
google-chrome --version
```

**Microsoft Edge:**
```bash
wget -qO- https://packages.microsoft.com/keys/microsoft.asc | gpg --dearmor | sudo tee /usr/share/keyrings/microsoft.gpg >/dev/null
echo "deb [signed-by=/usr/share/keyrings/microsoft.gpg] https://packages.microsoft.com/repos/edge stable main" | sudo tee /etc/apt/sources.list.d/microsoft-edge.list
sudo apt update && sudo apt install -y microsoft-edge-stable
```

**Chromium (snap):**
```bash
sudo snap install chromium
```

**Quickest (from the app):** `rust_teams --install-chromium` installs `chromium-browser` via `sudo apt-get`.

> Note: after installing any of these, the app automatically detects and uses it. No config needed.

---

## 4. Auto-update (in-app)

App checks GitHub Releases on startup. If new version found → downloads `rust_teams-linux-x64.tar.gz`, verifies SHA256, replaces binary, restarts.

**Requires:** binary installed in user-writable location (`/usr/local/bin` with sudo, or `~/.local/bin` without sudo).

---

## 5. Troubleshooting

| Symptom | Fix |
|---------|-----|
| `Failed to initialize GTK` on headless server | Normal — GUI app needs display. Use Xvfb or run on desktop. |
| `libwebkit2gtk-4.1.so.0: cannot open shared object` | `sudo apt install libwebkit2gtk-4.1-0` |
| Tray icon missing on GNOME | Install `gnome-shell-extension-appindicator` and enable extension |
| Wayland flicker / missing icons | `export GTK_BACKEND=x11` before launch, or use X11 session |
| Auto-update fails (permission denied) | Install binary in `~/.local/bin` or run once with `sudo` to update |
| WebKitGTK WebProcess crashes | `export WEBKIT_DISABLE_DMABUF_RENDERER=1` |
| White/blank screen on VDI (no GPU) | `export WEBKIT_DISABLE_DMABUF_RENDERER=1` or `export LIBGL_ALWAYS_SOFTWARE=true` |
| Login page renders all-white or hard to see | `export WEBKIT_DISABLE_COMPOSITING_MODE=1` before launch |
| Teams shows blank white page (WebKitGTK 2.40 too old) | Install Google Chrome/Chromium/Edge — the app auto-launches Teams in a Chromium app-mode window |
| Force Chromium backend | `rust_teams --backend chromium` |
| Force WebView embed | `rust_teams --backend webkit` |
| Install a system browser | On a machine with no browser, run `rust_teams --install-chromium` (installs `chromium-browser` via `sudo apt-get`). |

---

## 6. System requirements

| Component | Minimum |
|-----------|---------|
| OS | Ubuntu 22.04+ (or any distro with WebKitGTK 4.1) |
| Arch | x86_64 (binary) · ARM64 builds from source |
| RAM | 512 MB free (WebView2 ~200-400 MB) |
| Disk | ~10 MB binary + config |
| Display | Wayland or X11 (GTK 3 backend) |
| Audio | PulseAudio / PipeWire (for calls) |

---

## 7. Uninstall

**If installed via .deb:**
```bash
sudo apt remove rust-teams
rm -rf ~/.config/rust_teams
```

**If installed manually:**
```bash
sudo rm /usr/local/bin/rust-teams
rm -rf ~/.config/rust_teams
rm ~/.local/share/applications/rust-teams.desktop 2>/dev/null
```

---

## 8. Links

- Releases: https://github.com/thichcode/rust_teams/releases
- Issues: https://github.com/thichcode/rust_teams/issues
- Source: https://github.com/thichcode/rust_teams
