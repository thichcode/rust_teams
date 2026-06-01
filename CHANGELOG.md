# Changelog

All notable changes to R Teams.

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
