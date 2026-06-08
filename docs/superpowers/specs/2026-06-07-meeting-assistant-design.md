# R Teams Meeting Assistant — Design Spec

Status: Draft
Date: 2026-06-07

## Overview

Extract all meeting-related features (audio capture, STT, translation, suggestions, meeting notes) from the main R Teams app into a standalone native Windows GUI application built with Rust + egui/eframe.

## Goals

- Separate meeting features into an independent `.exe` that runs standalone
- Only local providers: whisper.cpp (STT) + Ollama (translation, suggestions, summarization)
- Own native UI window via egui (no WebView2 dependency)
- Communicate with main app via named pipe for optional realtime data sharing
- Config file independent from main app

## Non-goals

- Cloud providers (OpenAI, Google Translate, DeepL) — not in scope
- WebView2 dependency — mini app uses native egui rendering
- Replacing main app's meeting features — mini app is complementary

## Architecture

### Project structure

```
rteams-meeting-assistant/
├── Cargo.toml
└── src/
    ├── main.rs              # eframe entry point
    ├── app.rs               # MeetingAssistantApp (egui::App)
    ├── audio.rs             # AudioCapture (WASAPI loopback + cpal mic)
    ├── stt.rs               # LocalWhisper (whisper.cpp subprocess)
    ├── translate.rs         # OllamaTranslator
    ├── suggest.rs           # OllamaSuggester
    ├── notes.rs             # MeetingNotesGenerator
    ├── config.rs            # Config loading/saving
    ├── pipe.rs              # Named pipe IPC with main app
    └── ui/
        ├── mod.rs
        ├── transcript_panel.rs
        ├── translate_panel.rs
        ├── suggestions_panel.rs
        ├── notes_panel.rs
        └── config_panel.rs
```

### Data flow

```
WASAPI Loopback + Microphone
        │
        ▼
  AudioCapture (buffer 5s chunks)
        │
        ▼
  LocalWhisper (whisper.cpp subprocess → text)
        │
        ├──▶ Display in Transcript panel (egui)
        │
        ▼
  OllamaTranslator (POST /api/generate → translated text)
        │
        ├──▶ Display in Translation panel (egui)
        │
        ▼
  OllamaSuggester (context + latest → 3 suggestions)
        │
        ├──▶ Display in Suggestions panel (clickable buttons)
        │
        ▼
  MeetingNotesGenerator (if recording → Ollama summarize → .md file)

  Named pipe (optional) ──▶ main app (realtime display)
```

## Components

### Audio Capture (`audio.rs`)

- WASAPI loopback (system audio) via `wasapi` crate
- Microphone via `cpal` input stream
- Buffer accumulates ~5s of mixed float32 samples
- Same pattern as main app: `!Send` `Stream` handled in dedicated thread with per-thread tokio runtime

### STT (`stt.rs`)

- `LocalWhisper` only: writes WAV temp file, spawns `whisper.cpp -f <tmp.wav> -otxt`, reads stdout
- Uses `tokio::process::Command` with timeout

### Translator (`translate.rs`)

- `OllamaTranslator` only: `POST /api/generate` to Ollama API
- Async HTTP via `reqwest`

### Suggester (`suggest.rs`)

- `OllamaSuggester` only: generates 3 short reply candidates
- Rolling context: last 10 transcript lines

### Meeting Notes (`notes.rs`)

- Records all transcript → summarized via Ollama when user triggers "Generate"
- Saves as `.md` file to configurable directory
- Same logic as main app's `MeetingNotesGenerator`

### Config (`config.rs`)

File: `{data_dir}/RTeamsMeetingAssistant/config.json` (via `directories` crate → `%APPDATA%/RTeamsMeetingAssistant/` on Windows)

```json
{
  "ollama_endpoint": "http://localhost:11434",
  "whisper_binary": "C:\\Users\\<user>\\AppData\\Local\\RTeams\\whisper\\main.exe",
  "whisper_model": "C:\\Users\\<user>\\AppData\\Local\\RTeams\\whisper\\ggml-base.en.bin",
  "source_lang": "en",
  "target_lang": "vi",
  "translator_model": "qwen2.5:7b",
  "suggester_model": "gemma3:4b",
  "notes_dir": "%USERPROFILE%/Documents/MeetingNotes/",
  "auto_record": false
}
```

- Loaded on startup, saved on changes
- Serialized with serde

### Named Pipe (`pipe.rs`)

- Mini app creates server: `\\.\pipe\rteams-meet-assistant`
- Sends JSON messages to main app:
  ```json
  {"type": "transcript", "source_text": "...", "translated_text": "...", "suggestions": [...], "timestamp": 1234567890}
  ```
- Accepts commands from main app:
  ```json
  {"type": "ping"}
  {"type": "shutdown"}
  ```
- Non-blocking: pipe writes on background thread, failures silently retried

### UI (`ui/`)

#### Layout (egui)

```
┌─────────────────────────────────────────────────┐
│ ⚡ R Teams Meeting Assistant               _ □ X │
├─────────────────────┬───────────────────────────┤
│  TRANSCRIPT         │  TRANSLATION              │
│  ┌───────────────┐  │  ┌─────────────────────┐  │
│  │ Hello everyone│  │  │ Xin chào mọi người  │  │
│  │ Today we...   │  │  │ Hôm nay chúng ta... │  │
│  │ Let's discuss │  │  │ Chúng ta bàn về...  │  │
│  │ ...           │  │  │                     │  │
│  └───────────────┘  │  └─────────────────────┘  │
│                     │                           │
│  SUGGESTIONS        │  MEETING NOTES            │
│  [Yes] [No] [Sure] │  [🔴 Rec] [📝 Gen] New.md │
├─────────────────────┴───────────────────────────┤
│ ● Listening (loopback)  vi ← en  │ ⚙ Config    │
└─────────────────────────────────────────────────┘
```

#### Panels

- **Transcript Panel**: scrolling list, auto-scroll to bottom, line numbers or timestamps
- **Translation Panel**: aligned with transcript lines
- **Suggestions Panel**: 3 buttons in a row, click → copy to clipboard
- **Notes Panel**: Record toggle button, Generate Summary button, list of saved notes
- **Config Panel**: collapsible section or modal — Ollama URL, whisper paths, language selection, model selection
- **Status Bar**: recording indicator (● green), provider health, source/target language

## Dependencies

```toml
[dependencies]
egui = "0.31"
eframe = "0.31"
cpal = "0.13"
wasapi = "0.20"
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
hound = "3.5"
uuid = { version = "1", features = ["v4"] }
tokio = { version = "1", features = ["full"] }
chrono = "0.4"
directories = "5.0"
```

Note: No `tao`, `wry`, `ollama-rs`, or `zip` dependencies.

## Error Handling

| Condition | Behavior |
|-----------|----------|
| Ollama not running | Red status: "Ollama unreachable at http://localhost:11434" |
| whisper binary missing | Error dialog + path hint on first attempt |
| No microphone | Gray status: "No microphone detected" |
| Pipeline thread panic | Auto-restart with 5s delay, log to stderr |
| Named pipe write failure | Silent retry every 5s, no user-facing error |
| STT timeout (>30s) | Skip chunk, log warning, continue next chunk |

## Build Output

`cargo build --release` → `target/release/rteams-meeting-assistant.exe` (~5MB, standalone)

## Implementation Phases

### Phase 1: Core Pipeline + Minimal UI
- Project scaffolding, Cargo.toml
- `audio.rs`, `stt.rs`, `translate.rs`, `suggest.rs` — ported from main app
- `config.rs` with load/save
- Minimal egui window showing transcript + translation only
- Status bar with Ollama health check

### Phase 2: Full UI + Meeting Notes
- All 4 panels (transcript, translate, suggestions, notes)
- Notes generation with Ollama summarization
- Save/load notes list
- Config panel (collapsible)

### Phase 3: Named Pipe + Main App Integration
- Named pipe server in mini app
- Client in main app to receive realtime data
- Auto-launch / tray integration (optional)

## Testing

- Unit tests for STT parsing, translate response parsing, config serialization
- Integration test: full pipeline with mock Ollama server
- egui UI testing via `egui_kittest` if stable enough, otherwise manual visual verification

## Open Questions (Decided)

- **Whisper download:** v0.1 assumes whisper binary + model exist (user ran main app first). Future: `WhisperDownloader` can be ported from main app in a follow-up.
- **Workspace:** Mini app lives in same workspace as main app (`members = ["rteams-meeting-assistant"]` in root `Cargo.toml`), but builds to its own binary.
