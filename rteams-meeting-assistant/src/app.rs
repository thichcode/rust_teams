use std::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::audio::AudioCapture;
use crate::config::Config;
use crate::notes::save_transcript;
use crate::stt::LocalWhisper;
use crate::stt::SttProvider;
use crate::suggest::OllamaSuggester;
use crate::suggest::Suggester;
use crate::translate::OllamaTranslator;
use crate::translate::Translator;

#[derive(Clone)]
pub struct RealtimePayload {
    pub source_text: String,
    pub translated_text: String,
    pub suggestions: Vec<String>,
}

pub struct MeetingAssistantApp {
    config: Config,
    is_recording: bool,

    stop_tx: Option<mpsc::Sender<()>>,
    audio_thread: Option<std::thread::JoinHandle<()>>,
    running: Arc<AtomicBool>,

    payload_rx: mpsc::Receiver<RealtimePayload>,
    payload_tx: mpsc::Sender<RealtimePayload>,

    transcript_history: Vec<String>,
    current_transcript: String,
    current_translation: String,
    current_suggestions: Vec<String>,
    status_message: String,

    saved_notes: Vec<std::path::PathBuf>,
    show_config: bool,
}

impl MeetingAssistantApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, config: Config) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            status_message: "Ready".to_string(),
            config,
            is_recording: false,
            stop_tx: None,
            audio_thread: None,
            running: Arc::new(AtomicBool::new(false)),
            payload_rx: rx,
            payload_tx: tx,
            transcript_history: Vec::new(),
            current_transcript: String::new(),
            current_translation: String::new(),
            current_suggestions: Vec::new(),
            saved_notes: Vec::new(),
            show_config: false,
        }
    }

    pub fn start_pipeline(&mut self) {
        if self.config.whisper_binary.is_empty() || self.config.whisper_model.is_empty() {
            self.status_message = "❌ Configure whisper paths first".to_string();
            return;
        }
        if !std::path::Path::new(&self.config.whisper_binary).exists() {
            self.status_message = format!("❌ Not found: {}", self.config.whisper_binary);
            return;
        }

        self.status_message = "Starting...".to_string();
        self.is_recording = true;
        let (stop_tx, stop_rx) = mpsc::channel();
        self.stop_tx = Some(stop_tx);
        let running = Arc::new(AtomicBool::new(true));
        self.running = running.clone();
        let tx = self.payload_tx.clone();
        let cfg = self.config.clone();

        let handle = std::thread::Builder::new()
            .name("audio-pipeline".to_string())
            .spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let mut audio = AudioCapture::new();
                if let Err(e) = audio.start() {
                    log::error!("audio start: {e}");
                    return;
                }
                let stt = LocalWhisper::new(&cfg.whisper_binary, &cfg.whisper_model);
                let translator =
                    OllamaTranslator::new(&cfg.ollama_endpoint, &cfg.translator_model);
                let suggester =
                    OllamaSuggester::new(&cfg.ollama_endpoint, &cfg.suggester_model);
                let mut rolling: Vec<String> = Vec::new();

                while running.load(Ordering::Relaxed) {
                    if stop_rx.try_recv().is_ok() {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5000));
                    if !running.load(Ordering::Relaxed) {
                        break;
                    }
                    let samples = audio.drain_buffer();
                    let needed = 16000 * 3;
                    if samples.len() < needed {
                        continue;
                    }

                    let result = rt.block_on(async {
                        let text = stt.transcribe(&samples, &cfg.source_lang).await?;
                        let translated = translator
                            .translate(&text, &cfg.source_lang, &cfg.target_lang)
                            .await
                            .unwrap_or_default();
                        let ctx = rolling.join("\n");
                        let suggestions = suggester
                            .suggest(&ctx, &text, &cfg.target_lang, 3)
                            .await
                            .unwrap_or_default();
                        rolling.push(text.clone());
                        if rolling.len() > 10 {
                            rolling.remove(0);
                        }
                        Ok::<_, anyhow::Error>((text, translated, suggestions))
                    });

                    if let Ok((text, translated, suggestions)) = result {
                        let _ = tx.send(RealtimePayload {
                            source_text: text,
                            translated_text: translated,
                            suggestions,
                        });
                    }
                }
                let _ = audio.stop();
            });

        match handle {
            Ok(h) => {
                self.audio_thread = Some(h);
                self.status_message = "● Listening".to_string();
            }
            Err(e) => {
                self.status_message = format!("❌ Thread error: {e}");
                self.is_recording = false;
            }
        }
    }

    pub fn stop_pipeline(&mut self) {
        self.status_message = "Stopping...".to_string();
        self.running.store(false, Ordering::Relaxed);
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(h) = self.audio_thread.take() {
            let _ = h.join();
        }
        self.is_recording = false;
        self.status_message = "Stopped".to_string();
    }
}

impl eframe::App for MeetingAssistantApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(payload) = self.payload_rx.try_recv() {
            self.current_transcript = payload.source_text.clone();
            self.current_translation = payload.translated_text;
            self.current_suggestions = payload.suggestions;
            self.transcript_history.push(payload.source_text);
            if self.transcript_history.len() > 50 {
                self.transcript_history.remove(0);
            }
        }

        egui::TopBottomPanel::top("title_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("⚡ R Teams Meeting Assistant");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙ Settings").clicked() {
                        self.show_config = !self.show_config;
                    }
                });
            });
        });

        if self.show_config {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.config_panel(ui);
            });
            return;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let avail = ui.available_size();

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("TRANSCRIPT").strong().size(14.0));
                    egui::ScrollArea::vertical()
                        .id_salt("transcript")
                        .max_height(avail.y - 100.0)
                        .show(ui, |ui| {
                            for line in self.transcript_history.iter().rev() {
                                ui.label(line);
                            }
                        });
                });

                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("TRANSLATION").strong().size(14.0));
                    egui::ScrollArea::vertical()
                        .id_salt("translation")
                        .max_height(avail.y - 200.0)
                        .show(ui, |ui| {
                            ui.label(&self.current_translation);
                        });

                    ui.separator();
                    ui.label(egui::RichText::new("SUGGESTIONS").strong().size(14.0));
                    ui.horizontal(|ui| {
                        for s in &self.current_suggestions {
                            if ui.button(s).clicked() {
                                // TODO: copy to clipboard phase 2
                            }
                        }
                    });
                });
            });
        });

        egui::TopBottomPanel::bottom("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let btn_label = if self.is_recording { "🔴 Stop" } else { "🟢 Start" };
                if ui.button(btn_label).clicked() {
                    if self.is_recording {
                        self.stop_pipeline();
                    } else {
                        self.start_pipeline();
                    }
                }

                if ui.button("📝 Save Transcript").clicked() {
                    if !self.transcript_history.is_empty() {
                        if let Ok(path) = save_transcript(
                            &self.transcript_history,
                            &self.config.notes_dir,
                        ) {
                            self.saved_notes.push(path.clone());
                            self.status_message =
                                format!("Saved: {}", path.file_name().unwrap().to_string_lossy());
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(&self.status_message);
                });
            });
        });

        ctx.request_repaint();
    }
}

impl MeetingAssistantApp {
    fn config_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.separator();

        ui.label("Ollama Endpoint:");
        ui.text_edit_singleline(&mut self.config.ollama_endpoint);

        ui.label("Whisper Binary Path:");
        ui.text_edit_singleline(&mut self.config.whisper_binary);

        ui.label("Whisper Model Path:");
        ui.text_edit_singleline(&mut self.config.whisper_model);

        ui.label("Source Language (e.g. en):");
        ui.text_edit_singleline(&mut self.config.source_lang);

        ui.label("Target Language (e.g. vi):");
        ui.text_edit_singleline(&mut self.config.target_lang);

        ui.label("Translator Model:");
        ui.text_edit_singleline(&mut self.config.translator_model);

        ui.label("Suggester Model:");
        ui.text_edit_singleline(&mut self.config.suggester_model);

        ui.label("Notes Directory:");
        ui.text_edit_singleline(&mut self.config.notes_dir);

        if ui.button("💾 Save & Back").clicked() {
            self.config.save();
            self.show_config = false;
        }

        ui.separator();
        ui.colored_label(egui::Color32::GRAY, "Config saved to:");
        ui.label(crate::config::Config::config_path().display().to_string());
    }
}
