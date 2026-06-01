//! Realtime translate pipeline
//! Captures audio chunks from system loopback during a call, transcribes
//! via STT, translates to target language, and generates suggested replies.

#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::audio::AudioCapture;
use super::config::SttConfig;
use super::realtime_config::RealtimeTranslateConfig;
use super::stt::{self, SttProvider};

/// A realtime caption + suggestion payload sent to the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimePayload {
    pub timestamp: u64,
    /// Source language text (e.g. English)
    pub source_text: String,
    /// Source language code
    pub source_lang: String,
    /// Translated text in target language
    pub translated_text: String,
    /// Target language code
    pub target_lang: String,
    /// Suggested replies the user can say
    pub suggestions: Vec<String>,
}

/// Pipeline that drives audio capture -> STT -> translate -> suggestions
pub struct RealtimeTranslatePipeline {
    config: RealtimeTranslateConfig,
    is_running: Arc<AtomicBool>,
    /// Last N chunks used to maintain rolling context for suggestions
    rolling_context: Arc<Mutex<Vec<String>>>,
    sender: mpsc::UnboundedSender<RealtimePayload>,
}

impl RealtimeTranslatePipeline {
    pub fn new(
        config: RealtimeTranslateConfig,
        sender: mpsc::UnboundedSender<RealtimePayload>,
    ) -> Self {
        Self {
            config,
            is_running: Arc::new(AtomicBool::new(false)),
            rolling_context: Arc::new(Mutex::new(Vec::new())),
            sender,
        }
    }

    /// Get runtime flag (clone of the Arc)
    pub fn running_flag(&self) -> Arc<AtomicBool> {
        self.is_running.clone()
    }

    /// Start the pipeline. Spawns a tokio task that:
    ///   1. Starts mic + loopback capture
    ///   2. Every chunk_duration_secs, drains the buffer, calls STT,
    ///      then translate + suggest, then sends the result through `sender`.
    pub fn start(&self) -> Result<()> {
        if self.is_running.load(Ordering::Relaxed) {
            return Ok(());
        }
        self.is_running.store(true, Ordering::Relaxed);

        let cfg = self.config.clone();
        let running = self.is_running.clone();
        let context = self.rolling_context.clone();
        let tx = self.sender.clone();

        // Audio handle runs on its own thread because cpal::Stream is !Send.
        // The AudioCapture (and the cpal::Stream inside it) is constructed
        // INSIDE the spawned thread so it never crosses thread boundaries.
        let (chunk_tx, mut chunk_rx) =
            mpsc::unbounded_channel::<Vec<f32>>();

        let chunk_secs = cfg.chunk_duration_secs.max(2);
        let audio_cfg = cfg.stt.clone().into_audio_config();
        let sample_rate = audio_cfg.sample_rate as usize;
        let samples_per_chunk = sample_rate * chunk_secs as usize;
        let running_audio = running.clone();

        std::thread::spawn(move || {
            let mut audio = match AudioCapture::new(audio_cfg) {
                Ok(a) => a,
                Err(e) => {
                    log::error!("[Realtime] Failed to create AudioCapture: {}", e);
                    return;
                }
            };
            if let Err(e) = audio.start_recording() {
                log::error!("[Realtime] Failed to start recording: {}", e);
                return;
            }

            let mut accumulated: Vec<f32> = Vec::with_capacity(samples_per_chunk * 2);

            while running_audio.load(Ordering::Relaxed) {
                let available = audio.drain_buffer();
                accumulated.extend_from_slice(&available);

                if accumulated.len() >= samples_per_chunk {
                    let chunk: Vec<f32> =
                        accumulated.drain(..samples_per_chunk).collect();
                    if chunk_tx.send(chunk).is_err() {
                        break;
                    }
                } else {
                    std::thread::sleep(Duration::from_millis(200));
                }
            }

            // Stream is dropped here when `audio` goes out of scope
            let _ = audio.stop_recording();
        });

        // Async task: STT -> translate -> suggest
        tokio::spawn(async move {
            // Reuse the existing STT factory by projecting the realtime
            // STT config into the legacy SttConfig shape.
            let stt_cfg = SttConfig {
                provider_type: cfg.stt.provider_type.clone(),
                api_url: cfg.stt.api_url.clone(),
                api_key: cfg.stt.api_key.clone(),
                model: cfg.stt.model.clone(),
            };
            let stt_provider: Box<dyn SttProvider> = stt::create_stt_provider(&stt_cfg);
            let translator = super::translator::create_translator(
                &cfg.translator,
            );
            let suggester = super::suggester::create_suggester(
                &cfg.suggester,
            );

            while let Some(chunk) = chunk_rx.recv().await {
                if !running.load(Ordering::Relaxed) {
                    break;
                }

                let cfg_ref = &cfg;
                let ctx_ref = context.clone();

                // 1) STT
                let source_text = match stt_provider
                    .transcribe(&chunk, &cfg_ref.source_lang)
                    .await
                {
                    Ok(t) => t,
                    Err(e) => {
                        log::warn!("[Realtime] STT error: {}", e);
                        continue;
                    }
                };

                let source_text = source_text.trim().to_string();
                if source_text.is_empty() {
                    continue;
                }

                // 2) Translate
                let translated = match translator
                    .translate(&source_text, &cfg_ref.source_lang, &cfg_ref.target_lang)
                    .await
                {
                    Ok(t) => t,
                    Err(e) => {
                        log::warn!("[Realtime] translate error: {}", e);
                        source_text.clone()
                    }
                };

                // 3) Suggest
                let ctx_snapshot = {
                    let guard = ctx_ref.lock().unwrap();
                    guard.join("\n")
                };
                let suggestions = match suggester
                    .suggest(
                        &ctx_snapshot,
                        &source_text,
                        &cfg_ref.target_lang,
                        cfg_ref.suggestion_count as usize,
                    )
                    .await
                {
                    Ok(s) => s,
                    Err(e) => {
                        log::warn!("[Realtime] suggest error: {}", e);
                        Vec::new()
                    }
                };

                // 4) Push into rolling context (cap to last 10 turns)
                {
                    let mut guard = ctx_ref.lock().unwrap();
                    guard.push(source_text.clone());
                    if guard.len() > 10 {
                        let drop_n = guard.len() - 10;
                        guard.drain(0..drop_n);
                    }
                }

                // 5) Ship to UI
                let payload = RealtimePayload {
                    timestamp: now_millis(),
                    source_text,
                    source_lang: cfg_ref.source_lang.clone(),
                    translated_text: translated,
                    target_lang: cfg_ref.target_lang.clone(),
                    suggestions,
                };
                if tx.send(payload).is_err() {
                    break;
                }
            }
        });

        Ok(())
    }

    /// Stop the pipeline
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::Relaxed);
    }
}

fn now_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Tiny trait used to map the realtime STT config to AudioCapture's config.
trait IntoAudioConfig {
    fn into_audio_config(&self) -> super::config::AudioConfig;
}

impl IntoAudioConfig for super::realtime_config::SttRealtimeConfig {
    fn into_audio_config(&self) -> super::config::AudioConfig {
        super::config::AudioConfig {
            record_system_audio: true,
            record_microphone: true,
            sample_rate: 16000,
            channels: 1,
        }
    }
}
