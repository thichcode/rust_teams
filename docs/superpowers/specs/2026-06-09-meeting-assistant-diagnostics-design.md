# Meeting Assistant Setup Diagnostics Design

## Summary

Add a setup diagnostics section to the R Teams Meeting Assistant mini app. Users can verify microphone capture, system audio loopback, Whisper, and Ollama before a meeting. The UI shows clear `OK`, `Warning`, and `Failed` results, repair hints, and a diagnostics log that can be copied for debugging.

## Goals

- Add one full `Test setup` action that checks the meeting pipeline before recording.
- Add individual test actions for `Test Mic`, `Test System Audio`, `Test Whisper`, and `Test Ollama`.
- Show status, concise message, repair hint, and detailed log output for each diagnostic.
- Add `Copy diagnostics` so users can copy the current diagnostic report.
- Block `Start` only when a known critical setup failure exists.

## Non-Goals

- Do not add persisted file logs in this version.
- Do not redesign the main meeting UI.
- Do not replace the current audio pipeline, VAD, STT, translate, or suggestion implementation.
- Do not add cloud providers; diagnostics cover the existing local Whisper and Ollama flow.

## User Experience

Diagnostics live in the existing Settings screen under a new `Setup Diagnostics` section.

The section contains:

- `Test setup`: runs the full pre-meeting check.
- `Test Mic`: checks microphone capture.
- `Test System Audio`: checks WASAPI loopback/system output capture.
- `Test Whisper`: records about 3 seconds of mic audio, runs Whisper, and shows the transcribed result.
- `Test Ollama`: checks Ollama reachability and both configured models.
- A result table/card list with one row per check.
- A `Diagnostics log` scroll area.
- `Copy diagnostics`: copies a formatted report to the clipboard.

Each result has:

- Status: `Not run`, `Running`, `OK`, `Warning`, or `Failed`.
- Message: short user-facing result.
- Hint: concrete repair suggestion when not OK.
- Details: appended to diagnostics log.

## Full Setup Check

`Test setup` runs checks sequentially to keep logs readable and avoid fighting over audio devices:

1. Validate Whisper binary and model paths exist.
2. Run a Whisper smoke test with generated short silent/low-volume WAV input to verify the binary and model can execute.
3. Check Ollama endpoint reachability via `/api/tags`.
4. Verify the configured translator and suggester models are listed by Ollama.
5. Send a tiny `/api/generate` prompt to the configured translator and suggester models to catch broken model loads.
6. Check microphone availability and sample capture for a short window.
7. Check system audio loopback availability and sample capture for a short window.

Mic and system audio failures are non-critical as long as at least one audio source is available. If both mic and system audio are unavailable, the app treats that as a critical setup failure because recording cannot produce useful input.

## Individual Checks

`Test Mic`:

- Opens the default input device.
- Captures a short sample window.
- Reports `OK` when non-empty sample data is observed.
- Reports `Warning` when capture works but the signal is near silent.
- Reports `Failed` when no input device exists or the stream cannot start.

`Test System Audio`:

- Starts WASAPI loopback capture for a short sample window.
- Reports `OK` when samples are observed.
- Reports `Warning` when loopback opens but the signal is silent, because the user may simply not be playing audio.
- Reports `Failed` when loopback cannot initialize.

`Test Whisper`:

- Validates paths first.
- Records about 3 seconds of mic audio.
- Runs Whisper with the configured source language.
- Reports `OK` when Whisper returns text.
- Reports `Warning` when Whisper runs but returns empty text.
- Reports `Failed` when binary/model/path/process execution fails.

`Test Ollama`:

- Calls `GET {endpoint}/api/tags`.
- Confirms both configured models are present.
- Sends a tiny `/api/generate` prompt to each configured model to catch broken model loads.
- Reports missing models with a hint to run `ollama pull <model>`.

## Start Blocking Rules

When `Start` is clicked, the app blocks only on known critical failures:

- Whisper binary path missing or file not found.
- Whisper model path missing or file not found.
- Last Whisper smoke/user test failed.
- Ollama endpoint unreachable from the last diagnostics run.
- Configured translator or suggester model missing from the last Ollama check.
- Last mic and system audio checks both failed.

The app does not block start for microphone or system audio warnings, or for a failure in only one audio source. It shows the warning in the status message and lets recording proceed because the other source may still be usable.

If no diagnostics have been run yet, `Start` keeps the existing lightweight path checks and does not force a full diagnostic run. This avoids delaying urgent meeting starts.

## Architecture

Add a new `diagnostics.rs` module in `rteams-meeting-assistant/src/`.

Core types:

- `DiagnosticKind`: `Mic`, `SystemAudio`, `Whisper`, `Ollama`.
- `DiagnosticStatus`: `NotRun`, `Running`, `Ok`, `Warning`, `Failed`.
- `DiagnosticResult`: kind, status, message, hint, details, timestamp.
- `DiagnosticsReport`: latest results plus combined log text.
- `DiagnosticsRunner`: synchronous public methods that run checks on a background thread from the UI.

`MeetingAssistantApp` owns diagnostics state:

- Latest `DiagnosticsReport`.
- A channel for receiving diagnostic events/results from background threads.
- A boolean for whether any diagnostic is running.

The UI never runs expensive checks directly. Button clicks spawn a background thread, send incremental results through a channel, and request repaint as results arrive.

## Data Flow

Button click:

1. UI marks relevant checks `Running`.
2. UI spawns a diagnostics thread with a cloned `Config`.
3. Diagnostics thread runs checks and sends result events.
4. App update loop drains events, updates result state, and appends log lines.
5. `Copy diagnostics` formats current config summary and result details into clipboard text.

`Start` flow:

1. Existing path validation runs first.
2. Diagnostics critical failures are checked if present.
3. If blocked, `status_message` explains the first critical issue and points to `Settings > Test setup`.
4. If only warnings exist, `status_message` shows a warning and recording starts.

## Error Handling

All diagnostics return user-actionable errors. Raw process output and HTTP errors go into the diagnostics log, not only the short message.

Examples:

- Whisper binary missing: hint `Click Download Whisper or select the correct whisper.exe path in Settings.`
- Whisper model missing: hint `Download the model or select the .bin model path in Settings.`
- Ollama unreachable: hint `Start Ollama and verify the endpoint, usually http://localhost:11434.`
- Ollama model missing: hint `Run ollama pull <model> or change the model in Settings.`
- Mic silent: hint `Check Windows input permissions, selected default mic, and input volume.`
- System audio silent: hint `Play meeting/audio output and test again; silence can be normal when no audio is playing.`

## Testing

Add unit tests for result classification and report formatting where possible.

Manual verification:

- `cargo check --package rteams-meeting-assistant`
- Open app, Settings, run `Test setup` with valid config.
- Run individual tests.
- Temporarily set invalid Whisper path and confirm `Start` is blocked.
- Temporarily set invalid Ollama endpoint/model and confirm clear failure/hint.
- Confirm mic/system audio warnings do not block `Start`.
- Confirm `Copy diagnostics` copies readable report text.

## Release

Target mini app version: `rteams-meeting-assistant` v0.4.4.

Use existing mini app release workflow with tag `rteams-meeting-assistant-v0.4.4` after implementation and verification.
