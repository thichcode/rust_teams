# Meeting Assistant Audio Settings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add configurable microphone input and a system audio capture toggle to the R Teams Meeting Assistant mini app.

**Architecture:** Extend `Config` with serde-backed audio settings. Update `AudioCapture` to choose a specific cpal input device by name and conditionally start WASAPI loopback. Wire Settings UI and diagnostics to the same config values.

**Tech Stack:** Rust 2024, eframe/egui, cpal, wasapi, serde.

---

## File Structure

- Modify: `rteams-meeting-assistant/src/config.rs`
  - Add `audio_input_device` and `capture_system_audio` with defaults and tests.
- Modify: `rteams-meeting-assistant/src/audio.rs`
  - Add input device listing, selected-device lookup, and `AudioCapture::new(audio_input_device, capture_system_audio)`.
- Modify: `rteams-meeting-assistant/src/app.rs`
  - Add Settings UI controls and pass config to `AudioCapture`.
- Modify: `rteams-meeting-assistant/src/diagnostics.rs`
  - Make mic/Whisper tests use selected input and system audio test respect the toggle.
- Modify: `rteams-meeting-assistant/Cargo.toml`
  - Bump version to `0.4.5`.

## Tasks

- [ ] Add config defaults/tests for old config compatibility.
- [ ] Add audio device helper functions and selected mic capture.
- [ ] Wire pipeline to `AudioCapture::new(&cfg.audio_input_device, cfg.capture_system_audio)`.
- [ ] Add Settings UI: dropdown, refresh button, checkbox.
- [ ] Update diagnostics to use selected mic and skip system audio when disabled.
- [ ] Run `cargo test --package rteams-meeting-assistant` and `cargo check --package rteams-meeting-assistant`.
- [ ] Commit implementation, push, tag `rteams-meeting-assistant-v0.4.5`, and confirm release workflow.

## Self-Review

- Spec coverage: all UX, config, pipeline, diagnostics, release requirements are covered.
- Placeholder scan: no TBD/TODO placeholders.
- Type consistency: config names match design: `audio_input_device`, `capture_system_audio`.
