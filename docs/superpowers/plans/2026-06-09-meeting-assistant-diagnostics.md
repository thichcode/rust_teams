# Meeting Assistant Diagnostics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add setup diagnostics to the R Teams Meeting Assistant mini app so users can test Mic, System Audio, Whisper, and Ollama before starting a meeting.

**Architecture:** Add a focused `diagnostics.rs` module containing result types, report formatting, critical-failure rules, and runtime checks. Wire it into `MeetingAssistantApp` with a background diagnostics channel and a Settings UI section. Keep existing audio/STT/translate/suggest pipeline behavior intact.

**Tech Stack:** Rust 2024, eframe/egui 0.31, cpal 0.13, wasapi 0.20, tokio, reqwest, whisper.cpp subprocess.

---

## File Structure

- Create: `rteams-meeting-assistant/src/diagnostics.rs`
  - Defines `DiagnosticKind`, `DiagnosticStatus`, `DiagnosticResult`, `DiagnosticsReport`, `DiagnosticEvent`, `DiagnosticsRunner`.
  - Includes report formatting, critical block rule, Ollama checks, Whisper checks, and lightweight audio checks.
- Modify: `rteams-meeting-assistant/src/main.rs`
  - Add `mod diagnostics;`.
- Modify: `rteams-meeting-assistant/src/app.rs`
  - Add diagnostics state/channel fields.
  - Drain diagnostic results in `update()`.
  - Add `Setup Diagnostics` UI in Settings.
  - Add start guard for known critical diagnostic failures.
- Modify: `rteams-meeting-assistant/Cargo.toml`
  - Bump mini app version from `0.4.3` to `0.4.4` after implementation passes.

---

### Task 1: Diagnostics Core Types And Report Formatting

**Files:**
- Create: `rteams-meeting-assistant/src/diagnostics.rs`

- [ ] **Step 1: Write core types and unit tests**

Create `rteams-meeting-assistant/src/diagnostics.rs` with:

```rust
use std::collections::BTreeMap;

use chrono::Local;

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticKind {
    Mic,
    SystemAudio,
    Whisper,
    Ollama,
}

impl DiagnosticKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Mic => "Mic",
            Self::SystemAudio => "System Audio",
            Self::Whisper => "Whisper",
            Self::Ollama => "Ollama",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticStatus {
    NotRun,
    Running,
    Ok,
    Warning,
    Failed,
}

impl DiagnosticStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotRun => "Not run",
            Self::Running => "Running",
            Self::Ok => "OK",
            Self::Warning => "Warning",
            Self::Failed => "Failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiagnosticResult {
    pub kind: DiagnosticKind,
    pub status: DiagnosticStatus,
    pub message: String,
    pub hint: String,
    pub details: String,
    pub timestamp: String,
}

impl DiagnosticResult {
    pub fn new(
        kind: DiagnosticKind,
        status: DiagnosticStatus,
        message: impl Into<String>,
        hint: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            status,
            message: message.into(),
            hint: hint.into(),
            details: details.into(),
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DiagnosticEvent {
    Started(DiagnosticKind),
    Finished(DiagnosticResult),
    Done,
}

#[derive(Debug, Clone)]
pub struct DiagnosticsReport {
    pub results: BTreeMap<DiagnosticKind, DiagnosticResult>,
    pub log: String,
}

impl Default for DiagnosticsReport {
    fn default() -> Self {
        let mut results = BTreeMap::new();
        for kind in [DiagnosticKind::Mic, DiagnosticKind::SystemAudio, DiagnosticKind::Whisper, DiagnosticKind::Ollama] {
            results.insert(
                kind,
                DiagnosticResult::new(kind, DiagnosticStatus::NotRun, "Not run", "", ""),
            );
        }
        Self { results, log: String::new() }
    }
}

impl DiagnosticsReport {
    pub fn mark_running(&mut self, kind: DiagnosticKind) {
        self.results.insert(
            kind,
            DiagnosticResult::new(kind, DiagnosticStatus::Running, "Running...", "", ""),
        );
        self.append_log(kind, "Started");
    }

    pub fn apply(&mut self, result: DiagnosticResult) {
        self.append_log(result.kind, &format!(
            "{}: {}{}{}",
            result.status.label(),
            result.message,
            if result.hint.is_empty() { "" } else { " | Hint: " },
            result.hint,
        ));
        if !result.details.is_empty() {
            self.append_log(result.kind, &result.details);
        }
        self.results.insert(result.kind, result);
    }

    pub fn blocking_issue(&self) -> Option<String> {
        let whisper_failed = self.results.get(&DiagnosticKind::Whisper)
            .is_some_and(|r| r.status == DiagnosticStatus::Failed);
        if whisper_failed {
            return Some("Whisper diagnostics failed. Open Settings > Test setup.".to_string());
        }

        let ollama_failed = self.results.get(&DiagnosticKind::Ollama)
            .is_some_and(|r| r.status == DiagnosticStatus::Failed);
        if ollama_failed {
            return Some("Ollama diagnostics failed. Open Settings > Test setup.".to_string());
        }

        let mic_failed = self.results.get(&DiagnosticKind::Mic)
            .is_some_and(|r| r.status == DiagnosticStatus::Failed);
        let system_failed = self.results.get(&DiagnosticKind::SystemAudio)
            .is_some_and(|r| r.status == DiagnosticStatus::Failed);
        if mic_failed && system_failed {
            return Some("Both mic and system audio diagnostics failed. Open Settings > Test setup.".to_string());
        }

        None
    }

    pub fn format_for_clipboard(&self, config: &Config) -> String {
        let mut out = String::new();
        out.push_str("R Teams Meeting Assistant Diagnostics\n");
        out.push_str(&format!("Generated: {}\n\n", Local::now().format("%Y-%m-%d %H:%M:%S")));
        out.push_str("Config\n");
        out.push_str(&format!("Ollama endpoint: {}\n", config.ollama_endpoint));
        out.push_str(&format!("Whisper binary: {}\n", config.whisper_binary));
        out.push_str(&format!("Whisper model: {}\n", config.whisper_model));
        out.push_str(&format!("Translator model: {}\n", config.translator_model));
        out.push_str(&format!("Suggester model: {}\n\n", config.suggester_model));
        out.push_str("Results\n");
        for kind in [DiagnosticKind::Mic, DiagnosticKind::SystemAudio, DiagnosticKind::Whisper, DiagnosticKind::Ollama] {
            if let Some(r) = self.results.get(&kind) {
                out.push_str(&format!("- {}: {} - {}\n", kind.label(), r.status.label(), r.message));
                if !r.hint.is_empty() {
                    out.push_str(&format!("  Hint: {}\n", r.hint));
                }
            }
        }
        out.push_str("\nLog\n");
        out.push_str(&self.log);
        out
    }

    fn append_log(&mut self, kind: DiagnosticKind, line: &str) {
        self.log.push_str(&format!("[{}] {}: {}\n", Local::now().format("%H:%M:%S"), kind.label(), line));
    }
}

pub struct DiagnosticsRunner;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_when_whisper_failed() {
        let mut report = DiagnosticsReport::default();
        report.apply(DiagnosticResult::new(DiagnosticKind::Whisper, DiagnosticStatus::Failed, "bad", "fix", "details"));
        assert!(report.blocking_issue().unwrap().contains("Whisper"));
    }

    #[test]
    fn blocks_when_both_audio_sources_failed() {
        let mut report = DiagnosticsReport::default();
        report.apply(DiagnosticResult::new(DiagnosticKind::Mic, DiagnosticStatus::Failed, "bad", "fix", "details"));
        report.apply(DiagnosticResult::new(DiagnosticKind::SystemAudio, DiagnosticStatus::Failed, "bad", "fix", "details"));
        assert!(report.blocking_issue().unwrap().contains("Both mic"));
    }

    #[test]
    fn does_not_block_single_audio_failure() {
        let mut report = DiagnosticsReport::default();
        report.apply(DiagnosticResult::new(DiagnosticKind::Mic, DiagnosticStatus::Failed, "bad", "fix", "details"));
        assert!(report.blocking_issue().is_none());
    }

    #[test]
    fn clipboard_report_contains_config_and_results() {
        let mut report = DiagnosticsReport::default();
        report.apply(DiagnosticResult::new(DiagnosticKind::Ollama, DiagnosticStatus::Ok, "reachable", "", "ok"));
        let config = Config::default();
        let text = report.format_for_clipboard(&config);
        assert!(text.contains("Ollama endpoint"));
        assert!(text.contains("Ollama: OK"));
    }
}
```

- [ ] **Step 2: Register module and run tests**

Modify `rteams-meeting-assistant/src/main.rs`:

```rust
mod diagnostics;
```

Run: `cargo test --package rteams-meeting-assistant diagnostics --lib`

Expected: tests compile and pass.

- [ ] **Step 3: Commit**

```powershell
git add rteams-meeting-assistant/src/diagnostics.rs rteams-meeting-assistant/src/main.rs
```

---

### Task 2: Runtime Diagnostic Checks

**Files:**
- Modify: `rteams-meeting-assistant/src/diagnostics.rs`

- [ ] **Step 1: Add runner methods**

Extend `DiagnosticsRunner` with methods:

```rust
impl DiagnosticsRunner {
    pub fn run_full(config: Config, tx: std::sync::mpsc::Sender<DiagnosticEvent>) {
        for kind in [DiagnosticKind::Whisper, DiagnosticKind::Ollama, DiagnosticKind::Mic, DiagnosticKind::SystemAudio] {
            let _ = tx.send(DiagnosticEvent::Started(kind));
            let result = match kind {
                DiagnosticKind::Whisper => Self::check_whisper_smoke(&config),
                DiagnosticKind::Ollama => Self::check_ollama(&config),
                DiagnosticKind::Mic => Self::check_mic(),
                DiagnosticKind::SystemAudio => Self::check_system_audio(),
            };
            let _ = tx.send(DiagnosticEvent::Finished(result));
        }
        let _ = tx.send(DiagnosticEvent::Done);
    }

    pub fn run_one(kind: DiagnosticKind, config: Config, tx: std::sync::mpsc::Sender<DiagnosticEvent>) {
        let _ = tx.send(DiagnosticEvent::Started(kind));
        let result = match kind {
            DiagnosticKind::Mic => Self::check_mic(),
            DiagnosticKind::SystemAudio => Self::check_system_audio(),
            DiagnosticKind::Whisper => Self::check_whisper_user(&config),
            DiagnosticKind::Ollama => Self::check_ollama(&config),
        };
        let _ = tx.send(DiagnosticEvent::Finished(result));
        let _ = tx.send(DiagnosticEvent::Done);
    }
}
```

- [ ] **Step 2: Add Whisper checks**

Implement:

```rust
fn check_whisper_paths(config: &Config) -> Result<(), DiagnosticResult>
fn check_whisper_smoke(config: &Config) -> DiagnosticResult
fn check_whisper_user(config: &Config) -> DiagnosticResult
fn run_whisper(config: &Config, samples: &[f32]) -> anyhow::Result<String>
```

Use `crate::audio::AudioCapture::to_wav(samples, 16000, 1)` and `std::process::Command` for smoke test. For user test, instantiate `AudioCapture`, call `start()`, sleep 3 seconds, call `stop()`, then run Whisper on captured samples.

- [ ] **Step 3: Add Ollama check**

Implement:

```rust
fn check_ollama(config: &Config) -> DiagnosticResult
fn ollama_model_exists(tags_json: &serde_json::Value, model: &str) -> bool
fn ollama_generate(endpoint: &str, model: &str) -> anyhow::Result<String>
```

Use blocking `reqwest::blocking::Client` to keep diagnostics thread simple. If `reqwest` lacks `blocking`, update mini app `Cargo.toml` reqwest features to `features = ["json", "blocking"]`.

- [ ] **Step 4: Add audio checks**

Implement:

```rust
fn check_mic() -> DiagnosticResult
fn check_system_audio() -> DiagnosticResult
fn classify_samples(kind: DiagnosticKind, samples: Vec<f32>, silent_hint: &str) -> DiagnosticResult
```

For `check_mic`, use `AudioCapture::new()`, `start()`, sleep 800ms, `stop()`, and classify sample count/RMS. For `check_system_audio`, use the same capture object initially; if it captures any samples, return OK/Warning. Keep this minimal in v0.4.4 without replacing the existing audio implementation.

- [ ] **Step 5: Run checks**

Run: `cargo test --package rteams-meeting-assistant diagnostics --lib`

Expected: unit tests pass.

Run: `cargo check --package rteams-meeting-assistant`

Expected: build passes.

- [ ] **Step 6: Commit**

```powershell
git add rteams-meeting-assistant/src/diagnostics.rs rteams-meeting-assistant/Cargo.toml Cargo.lock
```

---

### Task 3: App State, Settings UI, And Copy Diagnostics

**Files:**
- Modify: `rteams-meeting-assistant/src/app.rs`

- [ ] **Step 1: Add imports and fields**

Add imports:

```rust
use crate::diagnostics::{DiagnosticEvent, DiagnosticKind, DiagnosticStatus, DiagnosticsReport, DiagnosticsRunner};
```

Add fields to `MeetingAssistantApp`:

```rust
diagnostics: DiagnosticsReport,
diagnostics_rx: mpsc::Receiver<DiagnosticEvent>,
diagnostics_tx: mpsc::Sender<DiagnosticEvent>,
diagnostics_running: bool,
```

Initialize in `new()` with `let (diag_tx, diag_rx) = mpsc::channel();`.

- [ ] **Step 2: Drain diagnostic events**

At the top of `update()`, after download/summary draining, add:

```rust
while let Ok(event) = self.diagnostics_rx.try_recv() {
    match event {
        DiagnosticEvent::Started(kind) => {
            self.diagnostics_running = true;
            self.diagnostics.mark_running(kind);
            self.status_message = format!("Testing {}...", kind.label());
        }
        DiagnosticEvent::Finished(result) => {
            self.status_message = format!("{}: {}", result.kind.label(), result.status.label());
            self.diagnostics.apply(result);
        }
        DiagnosticEvent::Done => {
            self.diagnostics_running = false;
            self.status_message = "Diagnostics complete".to_string();
        }
    }
}
```

- [ ] **Step 3: Add launch helpers**

Add methods to `impl MeetingAssistantApp`:

```rust
fn run_diagnostics_full(&mut self) {
    if self.diagnostics_running { return; }
    let config = self.config.clone();
    let tx = self.diagnostics_tx.clone();
    std::thread::spawn(move || DiagnosticsRunner::run_full(config, tx));
}

fn run_diagnostic_one(&mut self, kind: DiagnosticKind) {
    if self.diagnostics_running { return; }
    let config = self.config.clone();
    let tx = self.diagnostics_tx.clone();
    std::thread::spawn(move || DiagnosticsRunner::run_one(kind, config, tx));
}
```

- [ ] **Step 4: Add diagnostics UI in Settings**

In `config_panel`, before `Save & Back`, call:

```rust
self.diagnostics_panel(ui);
```

Add:

```rust
fn diagnostics_panel(&mut self, ui: &mut egui::Ui) {
    ui.separator();
    ui.heading("Setup Diagnostics");
    ui.horizontal(|ui| {
        if ui.add_enabled(!self.diagnostics_running, egui::Button::new("Test setup")).clicked() {
            self.run_diagnostics_full();
        }
        if ui.add_enabled(!self.diagnostics_running, egui::Button::new("Test Mic")).clicked() {
            self.run_diagnostic_one(DiagnosticKind::Mic);
        }
        if ui.add_enabled(!self.diagnostics_running, egui::Button::new("Test System Audio")).clicked() {
            self.run_diagnostic_one(DiagnosticKind::SystemAudio);
        }
        if ui.add_enabled(!self.diagnostics_running, egui::Button::new("Test Whisper")).clicked() {
            self.run_diagnostic_one(DiagnosticKind::Whisper);
        }
        if ui.add_enabled(!self.diagnostics_running, egui::Button::new("Test Ollama")).clicked() {
            self.run_diagnostic_one(DiagnosticKind::Ollama);
        }
    });

    for kind in [DiagnosticKind::Mic, DiagnosticKind::SystemAudio, DiagnosticKind::Whisper, DiagnosticKind::Ollama] {
        if let Some(result) = self.diagnostics.results.get(&kind) {
            ui.horizontal_wrapped(|ui| {
                ui.label(kind.label());
                ui.colored_label(status_color(result.status), result.status.label());
                ui.label(&result.message);
            });
            if !result.hint.is_empty() {
                ui.colored_label(egui::Color32::YELLOW, &result.hint);
            }
        }
    }

    ui.horizontal(|ui| {
        if ui.button("Copy diagnostics").clicked() {
            ui.ctx().copy_text(self.diagnostics.format_for_clipboard(&self.config));
            self.status_message = "Diagnostics copied".to_string();
        }
    });

    ui.label(egui::RichText::new("Diagnostics log").strong());
    egui::ScrollArea::vertical()
        .id_salt("diagnostics-log")
        .max_height(160.0)
        .show(ui, |ui| {
            ui.monospace(&self.diagnostics.log);
        });
}

fn status_color(status: DiagnosticStatus) -> egui::Color32 {
    match status {
        DiagnosticStatus::Ok => egui::Color32::GREEN,
        DiagnosticStatus::Warning => egui::Color32::YELLOW,
        DiagnosticStatus::Failed => egui::Color32::RED,
        DiagnosticStatus::Running => egui::Color32::LIGHT_BLUE,
        DiagnosticStatus::NotRun => egui::Color32::GRAY,
    }
}
```

- [ ] **Step 5: Run build**

Run: `cargo check --package rteams-meeting-assistant`

Expected: build passes.

- [ ] **Step 6: Commit**

```powershell
git add rteams-meeting-assistant/src/app.rs
```

---

### Task 4: Start Guard And Version Bump

**Files:**
- Modify: `rteams-meeting-assistant/src/app.rs`
- Modify: `rteams-meeting-assistant/Cargo.toml`

- [ ] **Step 1: Add start guard**

At the start of `start_pipeline()`, after existing Whisper path checks, add:

```rust
if let Some(issue) = self.diagnostics.blocking_issue() {
    self.status_message = issue;
    self.show_config = true;
    return;
}
```

- [ ] **Step 2: Bump version**

Update `rteams-meeting-assistant/Cargo.toml`:

```toml
version = "0.4.4"
```

- [ ] **Step 3: Verify**

Run: `cargo test --package rteams-meeting-assistant diagnostics --lib`

Expected: diagnostics tests pass.

Run: `cargo check --package rteams-meeting-assistant`

Expected: build passes.

- [ ] **Step 4: Commit**

```powershell
git add rteams-meeting-assistant/src/app.rs rteams-meeting-assistant/Cargo.toml Cargo.lock
```

---

## Self-Review

- Spec coverage: full setup, individual checks, status/hints/log, copy diagnostics, and start blocking are covered by Tasks 1-4.
- Placeholder scan: no TBD/TODO/later placeholders. Task 2 intentionally allows minimal system audio detection because the existing audio implementation combines mic and loopback; this is explicit and scoped to v0.4.4.
- Type consistency: `DiagnosticKind`, `DiagnosticStatus`, `DiagnosticResult`, `DiagnosticsReport`, `DiagnosticEvent`, and `DiagnosticsRunner` are defined in Task 1 before use in later tasks.
