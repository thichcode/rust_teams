# Changelog

All notable changes to R Teams.

## [0.9.0] - 2026-06-05

### Added
- **Manual On/Off toggle** — translate pipeline runs independently of meetings
  - "🔴 Off" / "🟢 On" button replaces old "Start listening"
  - WASAPI loopback captures system audio from any app (Teams, Zoom, Discord, etc.)
  - Close button stops pipeline when On
  - Pipeline start/stop is fully manual — no auto-trigger on meeting detection
- **Auto-download whisper model** — when toggling On with local STT, whisper binary + model are downloaded automatically (~100MB)
  - Shows "Downloading whisper model..." status during download
  - Updates config paths after download completes
- **Floating command bar** — Telegram-style `/` commands in Teams
  - Position: fixed, top-left corner, opacity 0.4 → focus shows
  - 9 built-in commands: `/help`, `/status`, `/translate on|off`, `/meeting start|stop`, `/config`, `/clear`, `/time`, `/date`, `/hello`
  - Dropdown with filter, "thinking..." indicator, result display
  - Independent of Teams' built-in bot system

## [0.8.0] - 2026-06-04

### Added
- **Local LLM mode** — run STT, translation, and suggestions entirely offline (no API keys needed)
  - 3-step wizard picks models for each pipeline role (STT → Translator → Suggester)
  - Uses whisper.cpp for local STT (auto-downloaded on first use)
  - Uses Ollama for local translation & suggestions
  - Per-provider hybrid dropdown allows mixing local + cloud providers
  - `LocalPreset` in config with 7 fields: `stt_model`, `translator_model`, `suggester_model`, `ollama_endpoint`, `whisper_binary`, `whisper_model`, `last_checked`
  - `check_local_readiness()` verifies both Ollama and Whisper availability
  - `build_wizard_options()` returns available models with recommended defaults
  - IPC handlers: `local_setup_open` + `local_setup_apply`
  - "🖥 Local" button in panel opens the wizard modal
  - Result banner shows green "ready" or amber "partial" status
  - `docs/LOCAL_LLM.md` — user guide with setup instructions

## [0.7.5] - 2026-06-03

### Changed
- **Auto-read v2** rewritten — opens each unread chat, reads bottom-most visible message bubble, verifies keyword, clicks back on miss
  - 30s cycle, max 3 chats/cycle, 5-min cooldown per chat (WeakMap)
  - `isUserTyping()` guard (skip if reply input has focus or content)
  - Selectors: `[data-tid="chat-item"]`, `[aria-label*="unread"]`, `[data-tid="messageBodyContent"]`, `[data-tid="chat-header-back"]`
  - Falls back to `Alt+Left` if back button not found

## [0.7.4] - 2026-06-03

### Added
- **In-panel API key configuration** — "⚙ Configure" button opens modal with STT/Translator/Suggester password fields
  - Empty field = keep existing value (safe update)
  - `config_update` IPC handler → PanelState `"config_saved"` with config path
  - `ConfigManager::update_api_keys()` + `config_path()`

### Added
- **WASAPI loopback capture** — `wasapi` 0.20 crate + `src/meeting/loopback.rs`
  - `AUDCLNT_STREAMFLAGS_LOOPBACK` on default render device
  - 16kHz mono f32, `Arc<Mutex<Vec<f32>>>` shared buffer
  - `AudioCapture::start_recording()` spawns loopback thread when `record_system_audio=true`; cpal fallback kept

## [0.7.3] - 2026-06-02

### Fixed
- **Panel auto-show** rewritten to poll `document.body` with `setTimeout` 50ms
  - No longer fails silently when body isn't ready at init time
  - Retries once more at 200ms if first attempt fails

## [0.7.2] - 2026-06-02

### Added
- **Realtime translate UI feedback** — `PanelState` channel with state machine
  - States: `idle`, `listening`, `error`, `no_api_key`, `no_mic`, `stopped`
  - Visual indicators for each state in the panel status bar

## [0.7.1] - 2026-06-01

### Fixed
- **Double-click user no longer opens chat in new Edge window** — `with_new_window_req_handler` was returning `NewWindowResponse::Allow` for `teams.microsoft.com` URLs. Per wry 0.55 source, `Allow` triggers WebView2 default behavior which defers to default browser (no-op for R Teams). Now detects Teams pop-out patterns (`/l/chat/`, `/l/person/`, `/l/channel/`, `users=`) and opens them in a separate Edge window via `msedge.exe --new-window`.

## [0.7.0] - 2026-05-31

### Added
- **Balanced WebView2 memory optimization** — saves ~250MB RAM vs default WebView2.
  - 3 profiles: `safe` (~70MB), `balanced` (~250MB, default), `aggressive` (~350MB)
  - CLI override: `--memory-profile safe|balanced|aggressive|off`
  - 9 new Chromium flags: `--disable-gpu`, `--disable-background-networking`, `--disable-breakpad`, `--disable-sync`, `--disable-translate`, `--disable-extensions`, `--disable-component-update`, `--disable-domain-reliability`, `--disable-features=BackForwardCache`, `--disable-features=IsolateOrigins,site-per-process`
  - JS-level optimizations: preconnect hints, visibility-pause, idle GC hint, content-visibility
- `docs/MEMORY.md` — full flag reference with trade-off documentation

## [0.6.3] - 2026-05-31

### Added
- **Auto-download whisper.cpp + model on startup** — no manual setup needed
  - Downloads `whisper-bin-x64.zip` (v1.7.4) from GitHub releases
  - Downloads `ggml-tiny.en.bin` from HuggingFace
  - 3 retries, 5min timeout, idempotent
  - Stores in `%APPDATA%/RustTeams/whisper/`

## [0.6.2] - 2026-05-30

### Added
- **Local Whisper via whisper.cpp subprocess** — runs offline, no API key
  - `provider_type: "local"|"whisper-cpp"` triggers subprocess mode
  - `api_url` = binary path, `api_key` = model path
  - Also works with any OpenAI-compatible local server (whisper.cpp server, LocalAI)

## [0.6.1] - 2026-05-30

### Fixed
- **Realtime translate tokio runtime** — pipeline now creates `tokio::runtime::Runtime::new()` inside audio `std::thread` and uses `block_on` for async HTTP calls. `cpal::Stream` (`!Send`) created inside thread too.

## [0.6.0] - 2026-05-30

### Added
- **Realtime translate + next-sentence suggestions in calls** — live transcription + translation + reply suggestions during Teams calls
  - New modules: `meeting/realtime_config.rs`, `meeting/realtime.rs`, `meeting/translator.rs`, `meeting/suggester.rs`
  - Translators: OpenAI, Ollama, Google v2, DeepL
  - Suggesters: OpenAI, Ollama (rolling context, last 10 turns)
  - New UI: `ui/realtime_panel.rs` (draggable overlay)

## [0.5.3] - 2026-05-29

### Fixed
- **Call join** — route `/meet/`, `/call/`, `meetup-join`, `teams.live.com/meet` to system browser
- **Click hijack on Add member dialog** — added `isInsideDialog()` guard
- **Performance script** reduced to preconnect hints only

## [0.5.2] - 2026-05-28

### Fixed
- **Auto-read** rewritten to target `[data-tid*="preview-text"]`, strip `Sender:` prefix, verify unread badge, 10/batch, 60s interval

## [0.5.0] - 2026-05-27

### Added
- Code signing support (optional via GitHub Secrets)
