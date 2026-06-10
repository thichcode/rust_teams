# Meeting Assistant Audio Settings Design

## Summary

Add audio input settings to the R Teams Meeting Assistant mini app. Users can choose a microphone instead of always using the Windows default input device, and can disable system audio loopback capture when they only want microphone input.

## Goals

- Add `Audio Input` selection in Settings.
- Add `Capture System Audio` checkbox in Settings.
- Preserve current behavior by default: Windows default mic plus system audio loopback enabled.
- Use the selected microphone in the live meeting pipeline and diagnostics.
- Skip/disable system audio diagnostics when system audio capture is disabled.
- Release as `rteams-meeting-assistant` v0.4.5.

## Non-Goals

- Do not add output device selection for WASAPI loopback in this version.
- Do not redesign the app layout.
- Do not replace the existing audio capture pipeline.

## UX

Settings gains an `Audio` section:

- `Audio Input`: dropdown with `Default` plus available input devices from `cpal`.
- `Refresh Audio Devices`: reloads the input device list.
- `Capture System Audio`: checkbox, default checked.

If the configured microphone is missing, `Start` fails with a clear status message and asks the user to choose another input. If `Capture System Audio` is unchecked, the app only captures microphone audio and `Test System Audio` reports that the check is skipped by configuration.

## Architecture

`Config` adds:

- `audio_input_device: String`: empty string means default input device.
- `capture_system_audio: bool`: defaults to `true` for compatibility.

`AudioCapture` stores these settings and selects a cpal input device by exact name. The pipeline constructs `AudioCapture` from the current config.

Diagnostics uses the same config:

- `Test Mic` captures from the configured input device.
- `Test Whisper` records from the configured input device.
- `Test System Audio` returns `Warning`/skipped when system audio capture is disabled.

## Testing

- Unit test config defaults: old config without new fields keeps default mic and system audio enabled.
- Unit test device selection helper behavior where practical.
- `cargo test --package rteams-meeting-assistant`
- `cargo check --package rteams-meeting-assistant`
- Manual test: Settings shows audio input dropdown and checkbox.
- Manual test: disabling system audio still allows Start with mic-only capture.
