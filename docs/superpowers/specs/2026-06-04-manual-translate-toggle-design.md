# Manual Translate Toggle (On/Off) — Design Spec

## Overview

Replace the meeting-auto-triggered pipeline with a fully manual On/Off toggle button in the Realtime Translate panel. The user can start/stop STT + Translation + Suggestions at any time, even when not in a Teams meeting. The pipeline uses WASAPI loopback for system audio capture, enabling translation of any audio (Teams calls, Zoom, Discord, etc.).

## Motivation

- User needs to translate calls from non-Teams tools (Zoom, Discord, etc.)
- Auto-detection by meeting state is unreliable and inflexible
- Full manual control: user decides when to listen and translate

## Changes

### 1. Panel Always Visible

- Remove auto-panel-show on meeting detection (`meeting_state_changed` → no longer triggers `renderState`)
- Panel is initialized on page load and stays visible
- **Close button**: when pipeline is **On**, clicking Close **stops the pipeline first**, then hides panel. When **Off**, Close hides panel directly.
- Toggle (▾/▸) button still works to collapse/expand body
- **When pipeline is On, panel stays visible** — close stops pipeline, panel stays up until user confirms Off state

### 2. On/Off Button

Replace existing "Start listening" / "Stop listening" button with a single **On/Off toggle**:

- **State: Off** — button shows "🔴 Off" (grayed out). Pipeline is idle. No audio capture.
- **State: On** — button shows "🟢 On". Pipeline runs: WASAPI loopback → STT → Translate → Suggestions.
- Click toggles between On and Off.
- When toggling **On → Off**: stop pipeline, release audio resources.
- When toggling **Off → On**: start WASAPI loopback capture, run pipeline.
- Pre-flight check (whisper binary, Ollama status, API keys) runs when toggling On.
- If pre-flight fails, button bounces back to Off and shows error in status bar.

### 3. WASAPI Loopback as Default Audio Source

- When the user toggles On, the pipeline uses **WASAPI loopback** (`AUDCLNT_STREAMFLAGS_LOOPBACK` on default render device) — captures all system audio output.
- No microphone input needed for this mode.
- Audio is 16kHz mono f32 — same format as current cpal mic capture.
- The STT pipeline reads from `Arc<Mutex<Vec<f32>>>` shared buffer (same pattern as `loopback.rs`).

### 4. Meeting Detection No Longer Controls Pipeline

- `meeting_state_changed` IPC messages are still received for **meeting notes logging only** (if meeting notes feature is enabled).
- `meeting_state_changed` **no longer** starts/stops the realtime translate pipeline.
- The `realtime_toggle` IPC message is **replaced** by the On/Off toggle logic — no separate message type needed.

### 5. IPC Changes

**Removed:** `realtime_toggle` IPC message (replaced by `manual_toggle`).

**New:** `manual_toggle` IPC message with `{enabled: bool}`:
- JS sends: `{type: "manual_toggle", data: {enabled: true}}` to turn On
- JS sends: `{type: "manual_toggle", data: {enabled: false}}` to turn Off

**Modified behavior:**
- `meeting_state_changed` — only used for meeting notes duration tracking. No panel/pipeline side effects.
- Panel state flow: user clicks On → JS sends `manual_toggle{enabled:true}` → Rust starts pipeline → Rust sends `PanelState{state: "listening"}` or `"error"`.

### 6. JS Code Changes in `realtime_panel.rs`

```
- Replace "Start listening" / "Stop listening" button HTML with "🟢 On" / "🔴 Off"
- Replace click handler: toggle pipeline via IPC message
- On meeting_state_changed: no longer show/hide panel or start/stop pipeline
- renderState() still handles "listening", "error", "no_api_key" etc. for status display
- Ensure panel does not auto-show on meeting state events
```

### 7. Rust Code Changes

**`src/main.rs`:**
- In `meeting_state_changed` handler: remove pipeline start/stop. Keep only meeting notes duration tracking.
- Add handler for new IPC message type (e.g., `manual_pipeline_toggle` with `{enabled: bool}`):
  - `enabled: true` → start WASAPI loopback pipeline with `record_system_audio: true`
  - `enabled: false` → stop pipeline
- Set `record_system_audio = true` when starting from manual toggle (not meeting mode).

**`src/meeting/audio.rs` / `src/meeting/loopback.rs`:**
- Ensure `AudioCapture::start_recording()` with `record_system_audio=true` works independently of meeting state.
- No changes needed if loopback is already working — it already spawns WASAPI thread.

### 8. Error Handling

- **Whisper not found / Ollama not running** → show in status bar, button stays Off
- **WASAPI device error** → show "Audio device error" in status bar
- **Pipeline crash** → auto toggle Off, show error
- All errors rendered via existing `renderState(panel, 'error', message)`

### 9. Testing

- Toggle On → verify pipeline starts (WASAPI loopback thread spawned)
- Toggle Off → verify pipeline stops (thread joined, audio released)
- Toggle On when whisper missing → verify error state, button stays Off
- Toggle On when Ollama unavailable → verify partial warning
- Meeting start/end → verify no pipeline state change
- Panel close → pipeline still runs (panel can be reopened on reload or re-init)

## Non-Goals

- No system tray icon
- No keyboard shortcuts
- No persistent pipeline state across app restart
- No push-to-talk or device selection UI
- No per-app audio routing
