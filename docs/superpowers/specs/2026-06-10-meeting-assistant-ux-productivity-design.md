# R Teams Meeting Assistant — UX & Productivity Design

Status: Draft
Date: 2026-06-10

## Overview

Add 4 UX/productivity features to the mini app: export transcript, system tray, keyboard shortcuts, and per-line copy. No new external dependencies beyond `tray-icon` and `global-hotkey`.

## Features

### 1. Export Transcript

**Location:** Bottom bar + Notes tab

- **Export TXT:** `notes_dir/transcript-YYYYMMDD-HHMMSS.txt`
  - Each line: `[Speaker 1] Hello world`
  - Plain text with speaker labels
- **Export MD:** `notes_dir/transcript-YYYYMMDD-HHMMSS.md`
  - Markdown with speaker headings (e.g. `### Speaker 1`)
  - Timestamp header
- **Buttons:**
  - Bottom bar: "Export TXT" and "Export MD" next to "Save Transcript"
  - Notes tab: "Export TXT" and "Export MD" buttons
- Disabled when `transcript_history` is empty
- **Helper:** `fn export_txt(history: &[String], dir: &Path) -> PathBuf` / `fn export_md(...) -> PathBuf`
- ~30 lines new code in `app.rs`
- No new dependencies

### 2. System Tray

**Dependency:** `tray-icon = "0.19"`

- Show tray icon when app starts
- **Menu items:**
  - "Show/Hide" — toggle window visibility
  - "Stop Recording" — stops pipeline if running
  - "Quit" — exits app
- Minimize to tray instead of closing (override close behavior)
- **Architecture:**
  - New module: `src/tray.rs`
  - `TrayManager` struct with channels to communicate with app
  - Runs on its own thread with a `tray_icon::TrayIconBuilder`
- Window visibility controlled via `frame.set_visible(false)` / `true`
- App exits when user clicks "Quit" from tray

### 3. Keyboard Shortcuts

**Dependency:** `global-hotkey = "0.8"`

- **Hotkeys registered:**
  - `Ctrl+Space` — toggle recording start/stop (global, even when app not focused)
  - `Ctrl+S` — save transcript (app-focused only, use egui keyboard handler)
- **Architecture:**
  - New module: `src/hotkey.rs`
  - `HotkeyManager` registers/unregisters `GlobalHotkeyManager`
  - Sends events via `mpsc::Sender` to app's `update()` loop
  - Unregister on drop
- `Ctrl+S` handled in `update()` via `ctx.input_mut(|i| i.consume_keyboard_event(...))` — no global hotkey needed

### 4. Copy Individual Transcript Lines

**Location:** Transcript scroll area

- Each transcript line has a small "Copy" button on the right
- Uses `ctx.copy_text(line)` — same pattern as `suggestions_tab`
- Button is `egui::Button::new("Copy")` with minimal width
- ~10 lines new code in transcript area

## Files Changed

| File | Change |
|------|--------|
| `Cargo.toml` | Add `tray-icon = "0.19"`, `global-hotkey = "0.8"` |
| `src/app.rs` | Export buttons, copy button per line, handle hotkey events |
| `src/tray.rs` | New: TrayManager, tray icon + menu |
| `src/hotkey.rs` | New: HotkeyManager, global hotkey registration |
| `src/main.rs` | Init tray + hotkey, pass channels |

## Dependencies

```toml
tray-icon = "0.19"
global-hotkey = "0.8"
```

Both are small, well-maintained crates. No breaking changes expected.

## Testing

- Manual testing only (GUI + system tray + global hotkeys)
- `cargo check` will catch compilation errors
- Verify: hotkeys work when app is backgrounded, tray show/hide works, export files are valid
