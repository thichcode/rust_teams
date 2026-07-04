# Meeting Assistant UX & Productivity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add export transcript (TXT/MD), system tray, keyboard shortcuts, and per-line copy to the mini app.

**Architecture:** 4 independent features added to existing egui app. Tray and hotkeys use separate threads + mpsc channels to communicate with the app's update() loop. Export and copy are pure UI additions.

**Tech Stack:** egui/eframe 0.31, tray-icon 0.19, global-hotkey 0.8

---

### Task 1: Update Cargo.toml with new dependencies

**Files:**
- Modify: `rteams-meeting-assistant/Cargo.toml:6-26`

- [ ] **Step 1: Add dependencies**

```toml
# After the existing [dependencies] block, add:
tray-icon = "0.19"
global-hotkey = "0.8"
```

- [ ] **Step 2: Verify compilation**

Run: `cd rteams-meeting-assistant && cargo check`
Expected: Compilation succeeds (warnings about unused deps are OK).

- [ ] **Step 3: Commit**

```bash
git add rteams-meeting-assistant/Cargo.toml rteams-meeting-assistant/Cargo.lock
git commit -m "chore: add tray-icon and global-hotkey dependencies"
```

---

### Task 2: Add export transcript functions

**Files:**
- Create: `rteams-meeting-assistant/src/export.rs`

- [ ] **Step 1: Create export.rs with TXT and MD export**

```rust
use std::fs;
use std::path::{Path, PathBuf};
use chrono::Local;

pub fn export_txt(history: &[String], dir: &Path) -> PathBuf {
    let ts = Local::now().format("%Y%m%d-%H%M%S");
    let path = dir.join(format!("transcript-{ts}.txt"));
    let content = history.join("\n");
    fs::write(&path, &content).expect("write txt export");
    path
}

pub fn export_md(history: &[String], dir: &Path) -> PathBuf {
    let ts = Local::now().format("%Y-%m-%d %H:%M:%S");
    let filename = Local::now().format("%Y%m%d-%H%M%S");
    let path = dir.join(format!("transcript-{filename}.md"));
    let mut content = format!("# Transcript — {ts}\n\n");
    for line in history {
        if let Some(speaker_end) = line.find(']') {
            let label = &line[..speaker_end + 1];
            let text = &line[speaker_end + 1..];
            content.push_str(&format!("### {label}\n{text}\n\n"));
        } else {
            content.push_str(&format!("{line}\n\n"));
        }
    }
    fs::write(&path, &content).expect("write md export");
    path
}
```

- [ ] **Step 2: Add test for export**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_export_txt() {
        let dir = std::env::temp_dir().join("rteams-test-export");
        let _ = fs::create_dir_all(&dir);
        let history = vec![
            "[Speaker 1] Hello world".to_string(),
            "[Speaker 2] Hi there".to_string(),
        ];
        let path = export_txt(&history, &dir);
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("Hello world"));
        assert!(content.contains("Hi there"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_export_md() {
        let dir = std::env::temp_dir().join("rteams-test-export-md");
        let _ = fs::create_dir_all(&dir);
        let history = vec![
            "[Speaker 1] Hello world".to_string(),
            "[Speaker 2] Hi there".to_string(),
        ];
        let path = export_md(&history, &dir);
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("### [Speaker 1]"));
        assert!(content.contains("Hello world"));
        assert!(content.contains("### [Speaker 2]"));
        assert!(content.contains("Hi there"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_export_empty() {
        let dir = std::env::temp_dir().join("rteams-test-export-empty");
        let _ = fs::create_dir_all(&dir);
        let path = export_txt(&[], &dir);
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "");
        let _ = fs::remove_dir_all(&dir);
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd rteams-meeting-assistant && cargo test test_export`
Expected: 3 passed, 0 failed.

- [ ] **Step 4: Commit**

```bash
git add rteams-meeting-assistant/src/export.rs
git commit -m "feat: add transcript export (txt + md)"
```

---

### Task 3: Add per-line copy and export buttons to UI

**Files:**
- Modify: `rteams-meeting-assistant/src/app.rs` (multiple locations)
- Modify: `rteams-meeting-assistant/src/main.rs`
- Modify: `rteams-meeting-assistant/src/lib.rs` or `main.rs` to declare `mod export`

- [ ] **Step 1: Register export module in main.rs**

In `rteams-meeting-assistant/src/main.rs`, add after `mod diagnostics;`:
```rust
mod export;
```

- [ ] **Step 2: Modify transcript history scroll area to show copy button per line**

In `app.rs`, find the transcript scroll area (around line 390-407) and add a copy button per line:

Change:
```rust
egui::ScrollArea::vertical()
    .id_salt("transcript")
    .max_height(avail.y - 100.0)
    .show(ui, |ui| {
        for line in self.transcript_history.iter().rev() {
            if let Some(speaker_end) = line.find(']') {
                let label = &line[..speaker_end + 1];
                let text = &line[speaker_end + 1..];
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::LIGHT_BLUE, label);
                    ui.label(text);
                });
            } else {
                ui.label(line);
            }
        }
    });
```

To:
```rust
egui::ScrollArea::vertical()
    .id_salt("transcript")
    .max_height(avail.y - 100.0)
    .show(ui, |ui| {
        for line in self.transcript_history.iter().rev() {
            ui.horizontal(|ui| {
                if let Some(speaker_end) = line.find(']') {
                    let label = &line[..speaker_end + 1];
                    let text = &line[speaker_end + 1..];
                    ui.colored_label(egui::Color32::LIGHT_BLUE, label);
                    ui.label(text);
                } else {
                    ui.label(line);
                }
                if ui.small_button("Copy").clicked() {
                    ctx.copy_text(line.clone());
                }
            });
        }
    });
```

- [ ] **Step 3: Add export buttons to bottom bar**

In the bottom panel (around line 466-512), after the `Save Transcript` button block:

```rust
if ui.button("Export TXT").clicked() {
    if !self.transcript_history.is_empty() {
        let path = export::export_txt(&self.transcript_history, &self.config.notes_dir);
        self.status_message = format!("Exported: {}", path.file_name().unwrap().to_string_lossy());
    }
}
if ui.button("Export MD").clicked() {
    if !self.transcript_history.is_empty() {
        let path = export::export_md(&self.transcript_history, &self.config.notes_dir);
        self.status_message = format!("Exported: {}", path.file_name().unwrap().to_string_lossy());
    }
}
```

- [ ] **Step 4: Add export buttons to Notes tab**

In the `notes_tab` function (around line 541), in the button row next to "Refresh" and "Open Folder":

```rust
if ui.button("Export TXT").clicked() {
    if !app.transcript_history.is_empty() {
        let path = export::export_txt(&app.transcript_history, &app.config.notes_dir);
        app.status_message = format!("Exported: {}", path.file_name().unwrap().to_string_lossy());
    }
}
if ui.button("Export MD").clicked() {
    if !app.transcript_history.is_empty() {
        let path = export::export_md(&app.transcript_history, &app.config.notes_dir);
        app.status_message = format!("Exported: {}", path.file_name().unwrap().to_string_lossy());
    }
}
```

- [ ] **Step 5: Compile and test**

Run: `cd rteams-meeting-assistant && cargo check && cargo test`
Expected: check passes, all tests pass.

- [ ] **Step 6: Commit**

```bash
git add rteams-meeting-assistant/src/main.rs rteams-meeting-assistant/src/app.rs
git commit -m "feat: add per-line copy and export buttons"
```

---

### Task 4: Implement system tray

**Files:**
- Create: `rteams-meeting-assistant/src/tray.rs`

- [ ] **Step 1: Create tray.rs**

```rust
use std::sync::mpsc;

pub enum TrayCommand {
    ToggleVisibility,
    StopRecording,
    Quit,
}

pub struct TrayManager {
    _tray: tray_icon::TrayIcon,
    pub rx: mpsc::Receiver<TrayCommand>,
}

impl TrayManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        let menu = tray_icon::Menu::new();
        let show_hide = tray_icon::MenuItemBuilder::new()
            .text("Show/Hide")
            .build();
        let stop_rec = tray_icon::MenuItemBuilder::new()
            .text("Stop Recording")
            .build();
        let quit = tray_icon::MenuItemBuilder::new()
            .text("Quit")
            .build();

        menu.append_items(&[&show_hide, &stop_rec, &quit]).unwrap();

        let tx_clone = tx.clone();
        show_hide.set_on_click(move || {
            let _ = tx_clone.send(TrayCommand::ToggleVisibility);
        });

        let tx_clone = tx.clone();
        stop_rec.set_on_click(move || {
            let _ = tx_clone.send(TrayCommand::StopRecording);
        });

        let tx_clone = tx.clone();
        quit.set_on_click(move || {
            let _ = tx_clone.send(TrayCommand::Quit);
        });

        let icon = tray_icon::Icon::from_resource(101).unwrap_or_else(|_| {
            let rgba = vec![0u8; 64 * 64 * 4];
            tray_icon::Icon::from_rgba(rgba, 64, 64).unwrap()
        });

        let tray = tray_icon::TrayIconBuilder::new()
            .with_icon(icon)
            .with_menu(menu)
            .build();

        Self {
            _tray: tray,
            rx,
        }
    }
}
```

- [ ] **Step 2: Register tray module in main.rs**

In `main.rs`, add:
```rust
mod tray;
```

- [ ] **Step 3: Integrate tray in app.rs app state**

Add to `MeetingAssistantApp`:
```rust
tray_rx: Option<std::sync::mpsc::Receiver<tray::TrayCommand>>,
```

Initialize in `new()`:
```rust
tray_rx: None,
```

In `update()` main loop, before the panel rendering, add:
```rust
if let Some(ref rx) = self.tray_rx {
    while let Ok(cmd) = rx.try_recv() {
        match cmd {
            tray::TrayCommand::ToggleVisibility => {
                if let Some(frame) = _frame {
                    let vis = frame.info().visible;
                    frame.set_visible(!vis);
                }
            }
            tray::TrayCommand::StopRecording => {
                if self.is_recording {
                    self.stop_pipeline();
                }
            }
            tray::TrayCommand::Quit => {
                _frame.close();
            }
        }
    }
}
```

- [ ] **Step 4: Init tray in main.rs**

In `main()`, after creating `native_options`, add:
```rust
let tray = tray::TrayManager::new();
let tray_rx = tray.rx;
```

Then pass `tray_rx` to `MeetingAssistantApp::new()`.

Update `new()` signature to accept `Option<mpsc::Receiver<tray::TrayCommand>>`.

- [ ] **Step 5: Compile**

Run: `cd rteams-meeting-assistant && cargo check`
Note: `tray-icon` depends on win32 APIs, works on Windows only.

- [ ] **Step 6: Commit**

```bash
git add rteams-meeting-assistant/src/tray.rs rteams-meeting-assistant/src/main.rs rteams-meeting-assistant/src/app.rs
git commit -m "feat: add system tray with show/hide, stop, quit"
```

---

### Task 5: Implement keyboard shortcuts

**Files:**
- Create: `rteams-meeting-assistant/src/hotkey.rs`

- [ ] **Step 1: Create hotkey.rs**

```rust
use std::sync::mpsc;
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::{HotKey, Modifiers, Code};

pub enum HotkeyEvent {
    ToggleRecording,
}

pub struct HotkeyManager {
    _manager: GlobalHotKeyManager,
    pub rx: mpsc::Receiver<HotkeyEvent>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let manager = GlobalHotKeyManager::new().expect("global hotkey manager");

        let toggle_key = HotKey::new(Some(Modifiers::CONTROL), Code::Space);
        manager.register(toggle_key).expect("register ctrl+space");

        let tx_clone = tx.clone();
        manager.set_handler(move |event| {
            if event.state == global_hotkey::hotkey::HotKeyState::Pressed {
                let _ = tx_clone.send(HotkeyEvent::ToggleRecording);
            }
        });

        Self {
            _manager: manager,
            rx,
        }
    }
}
```

- [ ] **Step 2: Register hotkey module in main.rs**

```rust
mod hotkey;
```

- [ ] **Step 3: Integrate hotkey in app.rs**

Add to `MeetingAssistantApp`:
```rust
hotkey_rx: Option<std::sync::mpsc::Receiver<hotkey::HotkeyEvent>>,
```

Initialize:
```rust
hotkey_rx: None,
```

In `update()` loop, alongside tray handling:
```rust
if let Some(ref rx) = self.hotkey_rx {
    while let Ok(event) = rx.try_recv() {
        match event {
            hotkey::HotkeyEvent::ToggleRecording => {
                if self.is_recording {
                    self.stop_pipeline();
                } else {
                    self.start_pipeline();
                }
            }
        }
    }
}
```

- [ ] **Step 4: Handle Ctrl+S in-app (non-global)**

In `update()`, add before the request_repaint:
```rust
ctx.input_mut(|i| {
    if i.consume_keyboard_event(egui::KeyboardEvent {
        modifiers: egui::Modifiers { ctrl: true, ..Default::default() },
        physical_key: Some(egui::Key::S),
        ..Default::default()
    }).is_some() {
        if !self.transcript_history.is_empty() {
            let path = export::export_txt(&self.transcript_history, &self.config.notes_dir);
            self.status_message = format!("Saved: {}", path.file_name().unwrap().to_string_lossy());
        }
    }
});
```

- [ ] **Step 5: Init hotkey in main.rs**

```rust
let hotkey = hotkey::HotkeyManager::new();
let hotkey_rx = hotkey.rx;
```

Pass to `MeetingAssistantApp::new()`.

- [ ] **Step 6: Compile**

Run: `cd rteams-meeting-assistant && cargo check`
Expected: check passes.

- [ ] **Step 7: Commit**

```bash
git add rteams-meeting-assistant/src/hotkey.rs rteams-meeting-assistant/src/main.rs rteams-meeting-assistant/src/app.rs
git commit -m "feat: add global hotkey (Ctrl+Space) and in-app Ctrl+S"
```

---

### Task 6: Run full test suite

**Files:**
- Run tests

- [ ] **Step 1: Run all tests**

Run: `cd rteams-meeting-assistant && cargo test`
Expected: all tests pass (including the 3 export tests from Task 2).

- [ ] **Step 2: Final check**

Run: `cd rteams-meeting-assistant && cargo check`
Expected: zero warnings.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "chore: final verification pass"
```
