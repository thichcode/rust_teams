# Local-only LLM mode — Design spec

**Date:** 2026-06-04
**Status:** Approved (pending review)
**Target version:** v0.8.0

## Goal

Add a first-class "Local LLM" mode to R Teams so users can run the entire
realtime-translate pipeline (STT + translation + suggestions) with no
cloud calls, on their own machine. The mode is a one-click preset that
picks a `whisper.cpp` binary for STT and an Ollama-served model for
both translation and suggestions. The user picks their models through
a 3-step wizard; R Teams verifies the local stack is ready and persists
the choices in `config.json`.

## Motivation

Teams' own translate is cloud-only and the user cannot replace it.
R Teams already wires up `LocalWhisper` (whisper.cpp subprocess) and
`OllamaTranslator` / `OllamaSuggester`, but the user must hand-edit
`config.json` to enable them. The wizard closes that gap and makes
local the path of least resistance for privacy-conscious and
offline users.

## Non-goals

- GPU detection / acceleration tuning (Ollama does this itself).
- Multi-Ollama-endpoint failover.
- Whisper fine-tuning.
- TTS / read-aloud replies.
- Encrypting `LocalPreset` in `config.json` (it contains model names,
  not secrets).
- Bundling Ollama inside the R Teams installer (~700 MB, antivirus
  concerns, and a separate update cadence).

## Design decisions (resolved with user)

| # | Decision | Choice |
|---|---|---|
| 1 | First-run setup | Auto-download whisper binary + model; show install instructions for Ollama |
| 2 | Default models | User picks via wizard (no hard-coded default bundle) |
| 3 | Hybrid mode (mix local + cloud) | Per-provider dropdown, user mixes freely |
| 4 | Failure UX | Panel shows actionable hint, no auto-fallback to cloud |
| 5 | Implementation approach | Wizard inside the existing realtime-translate panel |

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│ Panel UI (realtime_panel.rs)                            │
│                                                          │
│  [⚙ Configure]   [🖥 Local mode (new)]  [▶ Start]      │
│                      │                                   │
│                      ▼                                   │
│              ┌─────────────────┐                          │
│              │  Local Wizard   │ (3 steps)               │
│              │  1. STT model   │                          │
│              │  2. Translator  │                          │
│              │  3. Suggester   │                          │
│              └────────┬────────┘                          │
│                       │ on finish                        │
│                       ▼                                   │
│              send IPC "local_setup_apply"                │
└───────────────────────┼─────────────────────────────────┘
                        ▼
┌─────────────────────────────────────────────────────────┐
│ Rust main.rs IPC handler "local_setup_apply"            │
│                                                          │
│  1. Parse wizard payload                                 │
│  2. Update RealtimeTranslateConfig via ConfigManager    │
│  3. Spawn local_provider_check task                     │
│     - Ollama: GET /api/tags (verify model exists)       │
│     - Whisper: fs::metadata on binary + model           │
│  4. Return result via PanelState                        │
└──────────────────────┬──────────────────────────────────┘
                       ▼
┌─────────────────────────────────────────────────────────┐
│ New module: src/meeting/local_check.rs                  │
│                                                          │
│  pub struct LocalReadiness {                            │
│      ollama: ProviderStatus,    // Ready | NotInstalled │
│      whisper: ProviderStatus,   // ...                   │
│  }                                                       │
│  pub async fn check_local_readiness(cfg) -> LocalReadiness │
│  pub fn build_wizard_options() -> WizardOptions         │
└─────────────────────────────────────────────────────────┘
```

## Data flow

### Happy path

```
JS Panel                Rust IPC             local_check.rs        Ollama / FS
   │                       │                       │                    │
   │ user clicks "🖥 Local mode"                                       │
   ├──────────────────────►│                       │                    │
   │  {type: "local_setup_open"}                     │                    │
   │                       │                       │                    │
   │                       │ build_wizard_options  │                    │
   │                       ├──────────────────────►│                    │
   │                       │                       │ fs::metadata       │
   │                       │                       ├───────────────────►│
   │                       │                       │ GET /api/tags      │
   │                       │                       ├───────────────────►│
   │                       │ ◄─── WizardOptions ───│                    │
   │  {type: "local_setup_open", data: {...}}        │                    │
   │◄──────────────────────┤                       │                    │
   │ render wizard 3 steps (no IPC)                  │                    │
   │                       │                       │                    │
   │ user clicks "Apply"   │                       │                    │
   ├──────────────────────►│                       │                    │
   │  {type: "local_setup_apply", data: {...}}      │                    │
   │                       │                       │                    │
   │                       │ ConfigManager         │                    │
   │                       │ .update_local_preset  │                    │
   │                       │                       │                    │
   │                       │ check_local_readiness │                    │
   │                       ├──────────────────────►│                    │
   │                       │                       │ GET /api/tags      │
   │                       │                       ├───────────────────►│
   │                       │ ◄─── LocalReadiness ──│                    │
   │                       │                       │                    │
   │  PanelState{state: "local_ready", ...}          │                    │
   │◄──────────────────────┤                       │                    │
   │ hide wizard, show "🖥 Local · ready"           │                    │
```

### Failure (Ollama not running)

```
JS                          Rust                          Ollama
 │                            │                             │
 │ local_setup_apply          │                             │
 ├───────────────────────────►│                             │
 │                            │ check_local_readiness      │
 │                            ├────────────────────────────►│
 │                            │ ◄── ECONNREFUSED            │
 │                            │                             │
 │  PanelState{state: "local_partial",                    │
 │   detail: {ollama: NotRunning, whisper: Ready,         │
 │            install_hint: "..."}}                       │
 │◄───────────────────────────┤                             │
```

## Components & files

| File | Status | Purpose | LoC |
|---|---|---|---|
| `src/meeting/local_check.rs` | NEW | Ollama API, Whisper check, wizard catalog, readiness | ~250 |
| `src/meeting/mod.rs` | edit | `pub mod local_check;` | +1 |
| `src/meeting/realtime_config.rs` | edit | `LocalPreset` field + `apply_local_preset()` | +50 |
| `src/config.rs` | edit | `update_local_preset(stt, trans, sug)` | +40 |
| `src/main.rs` | edit | IPC handlers `local_setup_open`, `local_setup_apply` | +80 |
| `src/ui/realtime_panel.rs` | edit | "🖥 Local mode" button + `showLocalWizard()` | +300 |
| `tests/local_check.rs` | NEW | Unit tests | ~150 |

## Module: `src/meeting/local_check.rs`

```rust
pub enum ProviderStatus {
    Ready { model: String },
    NotInstalled { install_url: String, hint: String },
    NotRunning { endpoint: String, install_hint: String },
    ModelMissing { endpoint: String, model: String, install_hint: String },
    WrongPath { expected: String, actual: String },
}

pub struct LocalReadiness {
    pub ollama: ProviderStatus,    // covers translator + suggester
    pub whisper: ProviderStatus,   // STT
}

pub struct ModelOption {
    pub id: String,
    pub label: String,
    pub size_mb: u32,
    pub recommended: bool,
    pub install_hint: String,
}

pub struct WizardOptions {
    pub stt: Vec<ModelOption>,            // whisper models
    pub translator: Vec<ModelOption>,     // ollama translator models
    pub suggester: Vec<ModelOption>,      // ollama suggester models
    pub whisper_binary_path: String,
    pub ollama_endpoint: String,
}

pub fn build_wizard_options(cfg: &RealtimeTranslateConfig) -> WizardOptions;
pub async fn check_local_readiness(cfg: &RealtimeTranslateConfig) -> LocalReadiness;
pub fn apply_local_preset(cfg: &mut RealtimeTranslateConfig, choices: &LocalChoices);
```

## Config additions (`realtime_config.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalPreset {
    pub stt_model: String,        // e.g. "ggml-base.en"
    pub translator_model: String, // e.g. "qwen2.5:7b"
    pub suggester_model: String,  // e.g. "gemma3:4b"
    pub ollama_endpoint: String,  // default "http://localhost:11434"
    pub whisper_binary: String,   // path to main.exe
    pub whisper_model: String,    // path to ggml-*.bin
    pub last_checked: Option<i64>,
}

impl RealtimeTranslateConfig {
    pub fn apply_local_preset(&mut self, preset: &LocalPreset);
}
```

## UI

### Panel header (existing button row, extended)

```
[⚙ Configure]  [🖥 Local]  [▶ Start]  [■ Stop]
```

Clicking `🖥 Local` for the first time → opens wizard.
Clicking again → re-opens wizard pre-filled with current choices.

### Wizard modal (3 steps, single modal, step nav)

```
┌──────────────────────────────────────────────────────┐
│                                                       │
│            Set up Local LLM mode (3/3)               │
│                                                       │
│            Pick your Suggester model                  │
│                                                       │
│            ┌──────────────────────────────┐           │
│            │ ○ Gemma 3 4B ⭐ RECOMMENDED  │           │
│            │   ~3.3 GB · fast on CPU       │           │
│            ├──────────────────────────────┤           │
│            │ ○ Qwen 2.5 7B                 │           │
│            │   ~4.7 GB · better quality    │           │
│            ├──────────────────────────────┤           │
│            │ ○ Llama 3.1 8B                │           │
│            │   ~4.9 GB · most polyglot     │           │
│            └──────────────────────────────┘           │
│                                                       │
│            Install command:                          │
│            ┌──────────────────────────────┐           │
│            │ ollama pull gemma3:4b  [📋]   │           │
│            └──────────────────────────────┘           │
│                                                       │
│            [Back]              [Apply ✓]              │
└──────────────────────────────────────────────────────┘
```

### Step 1: STT (Whisper)
3 options:
- `ggml-tiny.en` ~75 MB · fastest, English-only ⭐
- `ggml-base.en` ~150 MB · better English accuracy
- `ggml-small` ~460 MB · multilingual
Plus "Custom path" for power users.

### Step 2: Translator (Ollama)
Lists installed models (from `GET /api/tags`); plus "+ Browse library" link.
Pre-selected: first `recommended: true` match, else `qwen2.5:7b`.
Red banner if Ollama not running.

### Step 3: Suggester (Ollama)
Same as Step 2, defaults to `gemma3:4b`.

### Result states

| Outcome | Banner | Action |
|---|---|---|
| All 3 ready | Green toast 3 s | "🖥 Local · ready" badge |
| Ollama not running | Yellow sticky | "Open Ollama" + "Copy `ollama pull ...`" |
| Whisper binary missing | Yellow sticky | "Download now" button |
| Whisper model missing | Yellow sticky | Auto-download with progress bar |

`Start` button is disabled while `local_partial` is sticky.

## IPC contract

| Type | Direction | Payload | Response |
|---|---|---|---|
| `local_setup_open` | JS→Rust | `{}` | `PanelState{state: "local_wizard_options", detail: JSON(WizardOptions)}` |
| `local_setup_apply` | JS→Rust | `{stt, translator, suggester}` | `PanelState{state: "local_ready" \| "local_partial", detail: JSON(LocalReadiness)}` |

## Error matrix

| Failure | Detection | UX | Auto-retry |
|---|---|---|---|
| Ollama not installed | `GET /api/tags` → `ECONNREFUSED` | Red banner + "Open Ollama" + copy `ollama pull ...` | No |
| Ollama running, model missing | 200 OK, model absent in `models[]` | Yellow notice + Copy button | No |
| Whisper binary missing | `fs::metadata(bin).is_err()` | Yellow banner + "Download now" | No |
| Whisper model missing | `fs::metadata(model).is_err()` | Auto-download, progress bar | Yes (3x) |
| Wrong user path | `fs::metadata` → `NotFound` | "File not found" + Browse | No |
| Disk full during whisper download | `ENOSPC` mid-write | "Download failed: disk full" + Retry | Yes (3x) |
| Ollama returns invalid JSON | `serde_json` parse fails | "Ollama returned malformed response" | No |
| Config write fails | `ConfigManager::save()` Err | Panel state `error` | No |
| `Start` while `local_partial` | Pipeline `start()` pre-check | Panel state `error` with hint | No |

## Testing

### Unit tests (`tests/local_check.rs`)

```rust
#[test] fn build_wizard_options_returns_at_least_one_per_role();
#[test] fn wizard_options_mark_recommended_models();
#[test] fn apply_local_preset_swaps_all_three_providers();
#[test] fn apply_local_preset_preserves_other_settings();

#[tokio::test] async fn ollama_list_models_parses_real_response();
#[tokio::test] async fn ollama_list_models_handles_connection_refused();
#[tokio::test] async fn ollama_list_models_handles_invalid_json();

#[test] fn whisper_status_ready_when_both_paths_exist();
#[test] fn whisper_status_not_installed_when_binary_missing();
#[test] fn whisper_status_wrong_path_when_model_dirty();

#[tokio::test] async fn readiness_counts_ready_providers();

#[test] fn config_manager_round_trips_local_preset();
```

Mock Ollama server via `tokio::net::TcpListener`.

### Manual smoke test

- [ ] First run, no Ollama, no whisper → wizard step 1 OK, step 2 red banner, step 3 OK, Apply → yellow "partial"
- [ ] Install Ollama, `ollama pull llama3.2`, click Retry → green "ready"
- [ ] Click Start listening → transcribe via whisper, translate via Ollama, suggest via Ollama; DevTools shows no cloud requests
- [ ] Configure modal shows `provider_type: ollama` / `local` in dropdowns
- [ ] Quit + relaunch → wizard step 1 pre-fills with previous choices
- [ ] Reopen wizard → previous selections pre-checked
- [ ] Ollama endpoint = `http://localhost:99999` → Apply → "Cannot connect" within 2 s
- [ ] Whisper download interrupted → relaunch → wizard shows "Resume" / "Restart"
- [ ] Toggle Local off, switch to Cloud (OpenAI) → suggestions work via cloud again

## Performance budget

| Operation | Target |
|---|---|
| Wizard open → options rendered | < 500 ms |
| `GET /api/tags` | < 1 s (timeout 2 s) |
| Whisper binary check | < 50 ms |
| Apply + readiness total | < 3 s |
| Whisper model download progress | every 500 ms |

## Out-of-scope (future)

- GPU detection / tuning
- Multi-endpoint Ollama cluster
- Whisper fine-tuning
- TTS read-aloud
- Encrypted `LocalPreset`

## Open questions deferred to implementation

| Q | Recommendation |
|---|---|
| Add "Test STT" button in step 1 (record 3 s, transcribe)? | Skip for v0.8.0; add in v0.8.1 if requested |
| Auto-close wizard after Apply success? | Yes (less clicking) |
| Always show `🖥 Local` / `☁ Cloud` in header? | Yes (useful info) |
