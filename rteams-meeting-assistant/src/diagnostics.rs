use std::collections::BTreeMap;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use chrono::Local;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleRate, StreamConfig};
use wasapi::{
    AudioCaptureClient, Direction, Handle, SampleType, StreamMode, WaveFormat, get_default_device,
};

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
        for kind in [
            DiagnosticKind::Mic,
            DiagnosticKind::SystemAudio,
            DiagnosticKind::Whisper,
            DiagnosticKind::Ollama,
        ] {
            results.insert(
                kind,
                DiagnosticResult::new(kind, DiagnosticStatus::NotRun, "Not run", "", ""),
            );
        }
        Self {
            results,
            log: String::new(),
        }
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
        self.append_log(
            result.kind,
            &format!(
                "{}: {}{}{}",
                result.status.label(),
                result.message,
                if result.hint.is_empty() {
                    ""
                } else {
                    " | Hint: "
                },
                result.hint,
            ),
        );
        if !result.details.is_empty() {
            self.append_log(result.kind, &result.details);
        }
        self.results.insert(result.kind, result);
    }

    pub fn blocking_issue(&self) -> Option<String> {
        if self
            .results
            .get(&DiagnosticKind::Whisper)
            .is_some_and(|r| r.status == DiagnosticStatus::Failed)
        {
            return Some("Whisper diagnostics failed. Open Settings > Test setup.".to_string());
        }
        if self
            .results
            .get(&DiagnosticKind::Ollama)
            .is_some_and(|r| r.status == DiagnosticStatus::Failed)
        {
            return Some("Ollama diagnostics failed. Open Settings > Test setup.".to_string());
        }

        let mic_failed = self
            .results
            .get(&DiagnosticKind::Mic)
            .is_some_and(|r| r.status == DiagnosticStatus::Failed);
        let system_failed = self
            .results
            .get(&DiagnosticKind::SystemAudio)
            .is_some_and(|r| r.status == DiagnosticStatus::Failed);
        if mic_failed && system_failed {
            return Some(
                "Both mic and system audio diagnostics failed. Open Settings > Test setup."
                    .to_string(),
            );
        }

        None
    }

    pub fn format_for_clipboard(&self, config: &Config) -> String {
        let mut out = String::new();
        out.push_str("R Teams Meeting Assistant Diagnostics\n");
        out.push_str(&format!(
            "Generated: {}\n\n",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        ));
        out.push_str("Config\n");
        out.push_str(&format!("Ollama endpoint: {}\n", config.ollama_endpoint));
        out.push_str(&format!("Whisper binary: {}\n", config.whisper_binary));
        out.push_str(&format!("Whisper model: {}\n", config.whisper_model));
        out.push_str(&format!("Translator model: {}\n", config.translator_model));
        out.push_str(&format!("Suggester model: {}\n\n", config.suggester_model));
        out.push_str("Results\n");
        for kind in [
            DiagnosticKind::Mic,
            DiagnosticKind::SystemAudio,
            DiagnosticKind::Whisper,
            DiagnosticKind::Ollama,
        ] {
            if let Some(result) = self.results.get(&kind) {
                out.push_str(&format!(
                    "- {}: {} - {} ({})\n",
                    kind.label(),
                    result.status.label(),
                    result.message,
                    result.timestamp
                ));
                if !result.hint.is_empty() {
                    out.push_str(&format!("  Hint: {}\n", result.hint));
                }
            }
        }
        out.push_str("\nLog\n");
        out.push_str(&self.log);
        out
    }

    fn append_log(&mut self, kind: DiagnosticKind, line: &str) {
        self.log.push_str(&format!(
            "[{}] {}: {}\n",
            Local::now().format("%H:%M:%S"),
            kind.label(),
            line
        ));
    }
}

pub struct DiagnosticsRunner;

impl DiagnosticsRunner {
    pub fn run_full(config: Config, tx: mpsc::Sender<DiagnosticEvent>) {
        for kind in [
            DiagnosticKind::Whisper,
            DiagnosticKind::Ollama,
            DiagnosticKind::Mic,
            DiagnosticKind::SystemAudio,
        ] {
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

    pub fn run_one(kind: DiagnosticKind, config: Config, tx: mpsc::Sender<DiagnosticEvent>) {
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

    pub fn check_whisper_smoke(config: &Config) -> DiagnosticResult {
        if let Err(result) = check_whisper_paths(config) {
            return result;
        }

        let samples = vec![0.001_f32; 16000 / 2];
        match run_whisper(config, &samples) {
            Ok(text) => DiagnosticResult::new(
                DiagnosticKind::Whisper,
                DiagnosticStatus::Ok,
                "Whisper binary and model executed",
                "",
                format!(
                    "Smoke test output: {}",
                    if text.is_empty() { "(empty)" } else { &text }
                ),
            ),
            Err(e) => DiagnosticResult::new(
                DiagnosticKind::Whisper,
                DiagnosticStatus::Failed,
                "Whisper smoke test failed",
                "Click Download Whisper or select the correct whisper.exe/model path in Settings.",
                e.to_string(),
            ),
        }
    }

    fn check_whisper_user(config: &Config) -> DiagnosticResult {
        if let Err(result) = check_whisper_paths(config) {
            return result;
        }

        let samples = match capture_mic_samples(Duration::from_secs(3)) {
            Ok(samples) => samples,
            Err(e) => {
                return DiagnosticResult::new(
                    DiagnosticKind::Whisper,
                    DiagnosticStatus::Failed,
                    "Could not record mic audio for Whisper test",
                    "Check Windows input permissions, selected default mic, and input volume.",
                    e.to_string(),
                );
            }
        };

        if samples.is_empty() {
            return DiagnosticResult::new(
                DiagnosticKind::Whisper,
                DiagnosticStatus::Warning,
                "Recorded no mic audio for Whisper test",
                "Check Windows input permissions, selected default mic, and input volume.",
                "Mic capture returned 0 samples",
            );
        }

        match run_whisper(config, &samples) {
            Ok(text) if text.trim().is_empty() => DiagnosticResult::new(
                DiagnosticKind::Whisper,
                DiagnosticStatus::Warning,
                "Whisper ran but returned empty text",
                "Speak clearly for 3 seconds and run Test Whisper again.",
                "Whisper process succeeded with empty transcript",
            ),
            Ok(text) => DiagnosticResult::new(
                DiagnosticKind::Whisper,
                DiagnosticStatus::Ok,
                "Whisper transcribed mic audio",
                "",
                format!("Transcript: {text}"),
            ),
            Err(e) => DiagnosticResult::new(
                DiagnosticKind::Whisper,
                DiagnosticStatus::Failed,
                "Whisper user test failed",
                "Click Download Whisper or select the correct whisper.exe/model path in Settings.",
                e.to_string(),
            ),
        }
    }

    fn check_ollama(config: &Config) -> DiagnosticResult {
        let client = match reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
        {
            Ok(client) => client,
            Err(e) => {
                return DiagnosticResult::new(
                    DiagnosticKind::Ollama,
                    DiagnosticStatus::Failed,
                    "Could not create HTTP client",
                    "Restart the app and try Test Ollama again.",
                    e.to_string(),
                );
            }
        };

        let endpoint = config.ollama_endpoint.trim_end_matches('/');
        let tags_url = format!("{endpoint}/api/tags");
        let tags: serde_json::Value = match client.get(&tags_url).send() {
            Ok(resp) if resp.status().is_success() => match resp.json() {
                Ok(json) => json,
                Err(e) => {
                    return DiagnosticResult::new(
                        DiagnosticKind::Ollama,
                        DiagnosticStatus::Failed,
                        "Ollama returned invalid tags JSON",
                        "Restart Ollama and run Test Ollama again.",
                        e.to_string(),
                    );
                }
            },
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().unwrap_or_default();
                return DiagnosticResult::new(
                    DiagnosticKind::Ollama,
                    DiagnosticStatus::Failed,
                    "Ollama endpoint is not healthy",
                    "Start Ollama and verify the endpoint, usually http://localhost:11434.",
                    format!("GET {tags_url} -> {status}: {body}"),
                );
            }
            Err(e) => {
                return DiagnosticResult::new(
                    DiagnosticKind::Ollama,
                    DiagnosticStatus::Failed,
                    "Cannot reach Ollama endpoint",
                    "Start Ollama and verify the endpoint, usually http://localhost:11434.",
                    e.to_string(),
                );
            }
        };

        let mut missing = Vec::new();
        for model in [&config.translator_model, &config.suggester_model] {
            if !ollama_model_exists(&tags, model) {
                missing.push(model.clone());
            }
        }
        if !missing.is_empty() {
            return DiagnosticResult::new(
                DiagnosticKind::Ollama,
                DiagnosticStatus::Failed,
                format!("Missing Ollama model(s): {}", missing.join(", ")),
                format!(
                    "Run ollama pull {} or change the model in Settings.",
                    missing[0]
                ),
                tags.to_string(),
            );
        }

        let mut details = String::new();
        for model in [&config.translator_model, &config.suggester_model] {
            match ollama_generate(endpoint, model) {
                Ok(output) => details.push_str(&format!("Model {model} OK: {output}\n")),
                Err(e) => {
                    return DiagnosticResult::new(
                        DiagnosticKind::Ollama,
                        DiagnosticStatus::Failed,
                        format!("Ollama model failed to generate: {model}"),
                        format!("Run ollama pull {model} or restart Ollama."),
                        e.to_string(),
                    );
                }
            }
        }

        DiagnosticResult::new(
            DiagnosticKind::Ollama,
            DiagnosticStatus::Ok,
            "Ollama endpoint and models are ready",
            "",
            details,
        )
    }

    fn check_mic() -> DiagnosticResult {
        match capture_mic_samples(Duration::from_millis(900)) {
            Ok(samples) => classify_samples(
                DiagnosticKind::Mic,
                samples,
                "Check Windows input permissions, selected default mic, and input volume.",
            ),
            Err(e) => DiagnosticResult::new(
                DiagnosticKind::Mic,
                DiagnosticStatus::Failed,
                "Could not start microphone capture",
                "Check Windows input permissions, selected default mic, and input volume.",
                e.to_string(),
            ),
        }
    }

    fn check_system_audio() -> DiagnosticResult {
        match capture_system_audio_samples(Duration::from_millis(900)) {
            Ok(samples) => classify_samples(
                DiagnosticKind::SystemAudio,
                samples,
                "Play meeting/audio output and test again; silence can be normal when no audio is playing.",
            ),
            Err(e) => DiagnosticResult::new(
                DiagnosticKind::SystemAudio,
                DiagnosticStatus::Failed,
                "Could not start system audio loopback",
                "Check the default Windows output device and try playing audio before testing.",
                e.to_string(),
            ),
        }
    }
}

fn check_whisper_paths(config: &Config) -> Result<(), DiagnosticResult> {
    if config.whisper_binary.trim().is_empty() {
        return Err(DiagnosticResult::new(
            DiagnosticKind::Whisper,
            DiagnosticStatus::Failed,
            "Whisper binary path is empty",
            "Click Download Whisper or select the correct whisper.exe path in Settings.",
            "whisper_binary is empty",
        ));
    }
    if !std::path::Path::new(&config.whisper_binary).exists() {
        return Err(DiagnosticResult::new(
            DiagnosticKind::Whisper,
            DiagnosticStatus::Failed,
            "Whisper binary file not found",
            "Click Download Whisper or select the correct whisper.exe path in Settings.",
            format!("Not found: {}", config.whisper_binary),
        ));
    }
    if config.whisper_model.trim().is_empty() {
        return Err(DiagnosticResult::new(
            DiagnosticKind::Whisper,
            DiagnosticStatus::Failed,
            "Whisper model path is empty",
            "Download the model or select the .bin model path in Settings.",
            "whisper_model is empty",
        ));
    }
    if !std::path::Path::new(&config.whisper_model).exists() {
        return Err(DiagnosticResult::new(
            DiagnosticKind::Whisper,
            DiagnosticStatus::Failed,
            "Whisper model file not found",
            "Download the model or select the .bin model path in Settings.",
            format!("Not found: {}", config.whisper_model),
        ));
    }
    Ok(())
}

fn run_whisper(config: &Config, samples: &[f32]) -> anyhow::Result<String> {
    let wav = crate::audio::AudioCapture::to_wav(samples, 16000, 1)?;
    let tmp_wav = std::env::temp_dir().join(format!("rteams_diag_{}.wav", uuid::Uuid::new_v4()));
    let tmp_out = tmp_wav.with_extension("txt");
    std::fs::write(&tmp_wav, wav)?;

    let lang = if config.source_lang.trim().is_empty() {
        "auto"
    } else {
        config.source_lang.trim()
    };
    let output = std::process::Command::new(&config.whisper_binary)
        .arg("-m")
        .arg(&config.whisper_model)
        .arg("-f")
        .arg(tmp_wav.as_os_str())
        .arg("-otxt")
        .arg("-l")
        .arg(lang)
        .arg("--no-prints")
        .output()
        .map_err(|e| anyhow::anyhow!("whisper: {e}"));

    let _ = std::fs::remove_file(&tmp_wav);
    let output = output?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = std::fs::remove_file(&tmp_out);
        anyhow::bail!("whisper exit {}: {stderr}", output.status);
    }

    let mut text = String::new();
    if let Ok(mut file) = std::fs::File::open(&tmp_out) {
        let _ = file.read_to_string(&mut text);
    }
    let _ = std::fs::remove_file(&tmp_out);
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&output.stdout).to_string();
    }
    Ok(text.trim().to_string())
}

fn capture_mic_samples(duration: Duration) -> anyhow::Result<Vec<f32>> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("No default input device"))?;
    let cfg = StreamConfig {
        channels: 1,
        sample_rate: SampleRate(16000),
        buffer_size: cpal::BufferSize::Default,
    };
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let recording = Arc::new(AtomicBool::new(true));
    let buffer_clone = buffer.clone();
    let recording_clone = recording.clone();
    let stream = device.build_input_stream(
        &cfg,
        move |data: &[f32], _: &_| {
            if recording_clone.load(Ordering::Relaxed) {
                if let Ok(mut buf) = buffer_clone.lock() {
                    buf.extend_from_slice(data);
                }
            }
        },
        |e| log::error!("diagnostic mic stream error: {e}"),
    )?;
    stream.play()?;
    std::thread::sleep(duration);
    recording.store(false, Ordering::Relaxed);
    drop(stream);
    Ok(buffer
        .lock()
        .map(|mut b| std::mem::take(&mut *b))
        .unwrap_or_default())
}

fn capture_system_audio_samples(duration: Duration) -> anyhow::Result<Vec<f32>> {
    let _ = wasapi::initialize_mta();
    let device = get_default_device(&Direction::Render)?;
    let mut client = device.get_iaudioclient()?;
    let fmt = WaveFormat::new(32, 32, &SampleType::Float, 16000, 1, None);
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: 0,
    };
    client.initialize_client(&fmt, &Direction::Capture, &mode)?;
    let h_event: Handle = client.set_get_eventhandle()?;
    let cap: AudioCaptureClient = client.get_audiocaptureclient()?;
    client.start_stream()?;

    let started = std::time::Instant::now();
    let mut deque = std::collections::VecDeque::new();
    let mut samples = Vec::new();
    while started.elapsed() < duration {
        let _ = h_event.wait_for_event(100);
        loop {
            match cap.read_from_device_to_deque(&mut deque) {
                Ok(_) if deque.is_empty() => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        while deque.len() >= 4 {
            let b0 = deque.pop_front().unwrap_or(0);
            let b1 = deque.pop_front().unwrap_or(0);
            let b2 = deque.pop_front().unwrap_or(0);
            let b3 = deque.pop_front().unwrap_or(0);
            samples.push(f32::from_bits(u32::from_le_bytes([b0, b1, b2, b3])));
        }
    }
    let _ = client.stop_stream();
    Ok(samples)
}

fn classify_samples(
    kind: DiagnosticKind,
    samples: Vec<f32>,
    silent_hint: &str,
) -> DiagnosticResult {
    if samples.is_empty() {
        return DiagnosticResult::new(
            kind,
            DiagnosticStatus::Warning,
            "Capture opened but returned no samples",
            silent_hint,
            "0 samples captured",
        );
    }

    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    if rms < 0.002 {
        return DiagnosticResult::new(
            kind,
            DiagnosticStatus::Warning,
            "Capture works but signal is near silent",
            silent_hint,
            format!("{} samples, RMS {:.6}", samples.len(), rms),
        );
    }

    DiagnosticResult::new(
        kind,
        DiagnosticStatus::Ok,
        "Capture is receiving audio",
        "",
        format!("{} samples, RMS {:.6}", samples.len(), rms),
    )
}

fn ollama_model_exists(tags_json: &serde_json::Value, model: &str) -> bool {
    tags_json
        .get("models")
        .and_then(|m| m.as_array())
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("name").and_then(|n| n.as_str()))
        .any(|name| name == model || name.ends_with(&format!("/{model}")))
}

fn ollama_generate(endpoint: &str, model: &str) -> anyhow::Result<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let body = serde_json::json!({
        "model": model,
        "prompt": "Reply with OK.",
        "stream": false,
        "options": { "temperature": 0.0, "num_predict": 8 }
    });
    let url = format!("{}/api/generate", endpoint.trim_end_matches('/'));
    let resp = client.post(&url).json(&body).send()?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        anyhow::bail!("POST {url} -> {status}: {text}");
    }
    let json: serde_json::Value = resp.json()?;
    Ok(json["response"].as_str().unwrap_or("").trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_when_whisper_failed() {
        let mut report = DiagnosticsReport::default();
        report.apply(DiagnosticResult::new(
            DiagnosticKind::Whisper,
            DiagnosticStatus::Failed,
            "bad",
            "fix",
            "details",
        ));

        assert!(report.blocking_issue().unwrap().contains("Whisper"));
    }

    #[test]
    fn blocks_when_both_audio_sources_failed() {
        let mut report = DiagnosticsReport::default();
        report.apply(DiagnosticResult::new(
            DiagnosticKind::Mic,
            DiagnosticStatus::Failed,
            "bad",
            "fix",
            "details",
        ));
        report.apply(DiagnosticResult::new(
            DiagnosticKind::SystemAudio,
            DiagnosticStatus::Failed,
            "bad",
            "fix",
            "details",
        ));

        assert!(report.blocking_issue().unwrap().contains("Both mic"));
    }

    #[test]
    fn does_not_block_single_audio_failure() {
        let mut report = DiagnosticsReport::default();
        report.apply(DiagnosticResult::new(
            DiagnosticKind::Mic,
            DiagnosticStatus::Failed,
            "bad",
            "fix",
            "details",
        ));

        assert!(report.blocking_issue().is_none());
    }

    #[test]
    fn clipboard_report_contains_config_and_results() {
        let mut report = DiagnosticsReport::default();
        report.apply(DiagnosticResult::new(
            DiagnosticKind::Ollama,
            DiagnosticStatus::Ok,
            "reachable",
            "",
            "ok",
        ));
        let config = crate::config::Config::default();

        let text = report.format_for_clipboard(&config);

        assert!(text.contains("Ollama endpoint"));
        assert!(text.contains("Ollama: OK"));
    }

    #[test]
    fn ollama_model_exists_matches_name_or_prefixed_name() {
        let tags = serde_json::json!({
            "models": [
                { "name": "qwen2.5:7b" },
                { "name": "library/gemma3:4b" }
            ]
        });

        assert!(ollama_model_exists(&tags, "qwen2.5:7b"));
        assert!(ollama_model_exists(&tags, "gemma3:4b"));
        assert!(!ollama_model_exists(&tags, "missing:latest"));
    }

    #[test]
    fn classify_samples_warns_for_silent_audio() {
        let result = classify_samples(DiagnosticKind::Mic, vec![0.0; 1600], "raise volume");

        assert_eq!(result.status, DiagnosticStatus::Warning);
        assert!(result.hint.contains("raise volume"));
    }

    #[test]
    fn classify_samples_ok_for_audible_audio() {
        let result = classify_samples(DiagnosticKind::Mic, vec![0.2; 1600], "raise volume");

        assert_eq!(result.status, DiagnosticStatus::Ok);
    }

    #[test]
    fn whisper_missing_paths_are_failed() {
        let config = Config::default();

        let result = DiagnosticsRunner::check_whisper_smoke(&config);

        assert_eq!(result.status, DiagnosticStatus::Failed);
        assert!(result.message.contains("Whisper binary"));
    }
}
