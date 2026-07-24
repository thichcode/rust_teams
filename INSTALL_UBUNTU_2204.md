# Rust Teams — Ubuntu 22.04 Desktop Install Guide

Binary targets **x86_64 Linux** with WebKitGTK 4.1. Runs on Ubuntu 22.04+ Desktop (GNOME / Wayland / X11).

---

## 1. Install system dependencies

```bash
sudo apt update && sudo apt install -y \
  libgtk-3-0 \
  libwebkit2gtk-4.1-0 \
  libjavascriptcoregtk-4.1-0 \
  libsoup-3.0-0 \
  libappindicator3-1 \
  librsvg2-2 \
  libxkbcommon0 \
  libfontconfig1 \
  libdbus-1-3 \
  libasound2 \
  libpulse0 \
  libx11-6 \
  libxcomposite1 \
  libxdamage1 \
  libxfixes3 \
  libxrandr2 \
  libxi6 \
  libxrender1 \
  libxtst6 \
  libatk1.0-0 \
  libcairo2 \
  libcups2 \
  libdrm2 \
  libgbm1 \
  libgdk-pixbuf-2.0-0 \
  libglib2.0-0 \
  libnspr4 \
  libnss3 \
  libpango-1.0-0 \
  libxcb1 \
  libxshmfence1 \
  ca-certificates \
  fontconfig \
  libsecret-1-0
```

**Optional (tray indicator on GNOME):**

```bash
sudo apt install -y gnome-shell-extension-appindicator
gnome-extensions enable appindicatorsupport@rgcjonas.gmail.com
```

---

## 2. Download release binary

```bash
VERSION="v0.9.58"   # or latest from https://github.com/thichcode/rust_teams/releases
cd /tmp
wget "https://github.com/thichcode/rust_teams/releases/download/${VERSION}/rust_teams-linux-x64.tar.gz"
wget "https://github.com/thichcode/rust_teams/releases/download/${VERSION}/rust_teams-linux-x64.tar.gz.sha256"
sha256sum -c rust_teams-linux-x64.tar.gz.sha256 && echo "✅ checksum OK"
tar xzf rust_teams-linux-x64.tar.gz
chmod +x rust_teams-linux-x64
sudo mv rust_teams-linux-x64 /usr/local/bin/rust-teams
```

---

## 3. Verify runtime dependencies

```bash
ldd /usr/local/bin/rust-teams | grep -E 'not found|libwebkit2gtk|libgtk|libsoup|libjavascriptcore'
# Expect all found → no "not found" lines
```

---

## 4. Run

```bash
rust-teams
```

First launch:

- Opens Microsoft Teams Web (https://teams.microsoft.com)
- Sign in with your Microsoft account
- App stores config at `~/.config/rust_teams/config.json`

---

## 5. Desktop entry (optional)

```bash
cat > ~/.local/share/applications/rust-teams.desktop <<'EOF'
[Desktop Entry]
Name=Rust Teams
Comment=Lightweight Microsoft Teams client (WebKitGTK)
Exec=/usr/local/bin/rust-teams
Icon=rust-teams
Terminal=false
Type=Application
Categories=Network;InstantMessaging;Office;
StartupWMClass=rust_teams
EOF

# Optional icon
wget -q -O ~/.local/share/icons/hicolor/256x256/apps/rust-teams.png \
  "https://raw.githubusercontent.com/thichcode/rust_teams/main/src/assets/icon_256.png"
gtk-update-icon-cache -f ~/.local/share/icons/hicolor/
```

Now search "Rust Teams" in app launcher.

---

## 6. Auto-update (in-app)

App checks GitHub Releases on startup. If new version found → downloads `rust_teams-linux-x64.tar.gz`, verifies SHA256, replaces binary, restarts.

**Requires:** binary installed in user-writable location (`/usr/local/bin` with sudo, or `~/.local/bin` without sudo).

---

## 7. Build from source (Ubuntu 22.04)

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

## 8. CI build (GitHub Actions)

Workflow `.github/workflows/release.yml` job **build-linux** runs on `ubuntu-22.04`:

```yaml
- name: Install Linux dependencies
  run: |
    sudo apt-get update
    sudo apt-get install -y \
      libgtk-3-dev \
      libwebkit2gtk-4.1-dev \
      libjavascriptcoregtk-4.1-dev \
      libappindicator3-dev \
      librsvg2-dev \
      libsoup-3.0-dev \
      libxkbcommon-dev \
      libfontconfig1-dev \
      patchelf
- name: Build (release)
  run: cargo build --release
- name: Package
  run: |
    cp target/release/rust_teams rust_teams-linux-x64
    tar czf rust_teams-linux-x64.tar.gz rust_teams-linux-x64
    sha256sum rust_teams-linux-x64 > rust_teams-linux-x64.sha256
    sha256sum rust_teams-linux-x64.tar.gz > rust_teams-linux-x64.tar.gz.sha256
```

Artifacts uploaded as `rust_teams-linux-x64` + `rust_teams-linux-x64.tar.gz` + checksums.

---

## 9. Troubleshooting

| Symptom | Fix |
|---------|-----|
| `Failed to initialize GTK` on headless server | Normal — GUI app needs display. Use Xvfb or run on desktop. |
| `libwebkit2gtk-4.1.so.0: cannot open shared object` | `sudo apt install libwebkit2gtk-4.1-0` |
| Tray icon missing on GNOME | Install `gnome-shell-extension-appindicator` and enable extension |
| Wayland flicker / missing icons | `export GTK_BACKEND=x11` before launch, or use X11 session |
| Auto-update fails (permission denied) | Install binary in `~/.local/bin` or run once with `sudo` to update |
| WebKitGTK WebProcess crashes | `export WEBKIT_DISABLE_DMABUF_RENDERER=1` |

---

## 10. System requirements

| Component | Minimum |
|-----------|---------|
| OS | Ubuntu 22.04+ (or any distro with WebKitGTK 4.1) |
| Arch | x86_64 (binary) · ARM64 builds from source |
| RAM | 512 MB free (WebView2 ~200–400 MB) |
| Disk | ~10 MB binary + config |
| Display | Wayland or X11 (GTK 3 backend) |
| Audio | PulseAudio / PipeWire (for calls) |

---

## 11. Uninstall

```bash
sudo rm /usr/local/bin/rust-teams
rm -rf ~/.config/rust_teams
rm ~/.local/share/applications/rust-teams.desktop
rm ~/.local/share/icons/hicolor/256x256/apps/rust-teams.png
```

---

## 12. Links

- Releases: https://github.com/thichcode/rust_teams/releases
- Issues: https://github.com/thichcode/rust_teams/issues
- Source: https://github.com/thichcode/rust_teams