use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use crate::audio::AudioCapture;
use crate::config::Config;
use crate::diagnostics::{
    DiagnosticEvent, DiagnosticKind, DiagnosticStatus, DiagnosticsReport, DiagnosticsRunner,
};
use crate::diarize::Diarizer;
use crate::download::WhisperDownloader;
use crate::notes::{list_notes, save_transcript, spawn_summarize};
use crate::stt::LocalWhisper;
use crate::stt::SttProvider;
use crate::suggest::OllamaSuggester;
use crate::suggest::Suggester;
use crate::translate::OllamaTranslator;
use crate::translate::Translator;
use crate::vad::Vad;

#[derive(Clone, PartialEq)]
enum RightTab {
    Translation,
    Suggestions,
    Notes,
}

#[derive(Clone)]
pub struct RealtimePayload {
    pub speaker_label: String,
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
    right_tab: RightTab,

    summary_text: String,
    summary_generating: bool,
    summary_rx: mpsc::Receiver<String>,
    summary_tx: mpsc::Sender<String>,

    is_downloading: bool,
    download_rx: mpsc::Receiver<String>,
    download_tx: mpsc::Sender<String>,

    start_time: Option<std::time::Instant>,
    last_auto_save: std::time::Instant,

    diagnostics: DiagnosticsReport,
    diagnostics_rx: mpsc::Receiver<DiagnosticEvent>,
    diagnostics_tx: mpsc::Sender<DiagnosticEvent>,
    diagnostics_running: bool,
    audio_input_devices: Vec<String>,
}

impl MeetingAssistantApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, config: Config) -> Self {
        let (tx, rx) = mpsc::channel();
        let (stx, srx) = mpsc::channel();
        let (dtx, drx) = mpsc::channel();
        let (diag_tx, diag_rx) = mpsc::channel();
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
            right_tab: RightTab::Translation,
            summary_text: String::new(),
            summary_generating: false,
            summary_rx: srx,
            summary_tx: stx,
            is_downloading: false,
            download_rx: drx,
            download_tx: dtx,
            start_time: None,
            last_auto_save: std::time::Instant::now(),
            diagnostics: DiagnosticsReport::default(),
            diagnostics_rx: diag_rx,
            diagnostics_tx: diag_tx,
            diagnostics_running: false,
            audio_input_devices: AudioCapture::input_device_names(),
        }
    }

    pub fn start_pipeline(&mut self) {
        if self.config.whisper_binary.is_empty() || self.config.whisper_model.is_empty() {
            self.status_message = "Set whisper paths in Settings first".to_string();
            return;
        }
        if !std::path::Path::new(&self.config.whisper_binary).exists() {
            self.status_message = format!("Not found: {}", self.config.whisper_binary);
            return;
        }
        if self.config.whisper_model.is_empty() {
            self.status_message = "Set whisper model path in Settings first".to_string();
            return;
        }
        if !std::path::Path::new(&self.config.whisper_model).exists() {
            self.status_message = format!("Not found: {}", self.config.whisper_model);
            return;
        }
        if let Some(issue) = self.diagnostics.blocking_issue() {
            self.status_message = issue;
            self.show_config = true;
            return;
        }

        self.status_message = "Starting...".to_string();
        self.is_recording = true;
        self.start_time = Some(std::time::Instant::now());
        self.last_auto_save = std::time::Instant::now();
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
                let mut audio = AudioCapture::new(
                    &cfg.audio_input_device,
                    cfg.capture_system_audio,
                );
                if let Err(e) = audio.start() {
                    log::error!("audio start: {e}");
                    return;
                }

                let stt = LocalWhisper::new(&cfg.whisper_binary, &cfg.whisper_model);
                let translator = OllamaTranslator::new(&cfg.ollama_endpoint, &cfg.translator_model);
                let suggester = OllamaSuggester::new(&cfg.ollama_endpoint, &cfg.suggester_model);
                let mut vad = Vad::new();
                let mut diarizer = Diarizer::new();
                let mut rolling: Vec<String> = Vec::new();

                let mut utterance: Vec<f32> = Vec::new();
                let mut silence_frames: u32 = 0;
                const SILENCE_TIMEOUT: u32 = 15;

                while running.load(Ordering::Relaxed) {
                    if stop_rx.try_recv().is_ok() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                    if !running.load(Ordering::Relaxed) {
                        break;
                    }

                    let samples = audio.drain_buffer();
                    if samples.is_empty() {
                        continue;
                    }

                    let mut offset = 0;
                    while offset + 480 <= samples.len() {
                        let frame = &samples[offset..offset + 480];
                        offset += 480;
                        if vad.is_voice(frame) {
                            utterance.extend_from_slice(frame);
                            silence_frames = 0;
                        } else {
                            silence_frames += 1;
                        }
                    }

                    if silence_frames >= SILENCE_TIMEOUT && !utterance.is_empty() {
                        if utterance.len() >= 16000 {
                            let speaker = diarizer.next_utterance();
                            let samples_to_process = utterance.clone();
                            utterance.clear();

                            let result = rt.block_on(async {
                                let text = stt
                                    .transcribe(&samples_to_process, &cfg.source_lang)
                                    .await?;
                                let translated = translator
                                    .translate(&text, &cfg.source_lang, &cfg.target_lang)
                                    .await
                                    .unwrap_or_default();
                                let ctx = rolling.join("\n");
                                let suggestions = suggester
                                    .suggest(&ctx, &text, &cfg.source_lang, 3)
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
                                    speaker_label: speaker,
                                    source_text: text,
                                    translated_text: translated,
                                    suggestions,
                                });
                            }
                        } else {
                            utterance.clear();
                        }
                    }
                }
                let _ = audio.stop();
            });

        match handle {
            Ok(h) => {
                self.audio_thread = Some(h);
                self.status_message = "Listening".to_string();
            }
            Err(e) => {
                self.status_message = format!("Thread error: {e}");
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
        self.start_time = None;
        self.status_message = "Stopped".to_string();
    }

    fn start_download(&mut self) {
        if self.is_downloading {
            return;
        }
        self.is_downloading = true;
        self.status_message = "Downloading whisper...".to_string();
        let data_dir = directories::ProjectDirs::from("com", "rteams", "RTeamsMeetingAssistant")
            .map(|p| p.data_dir().to_path_buf())
            .unwrap_or_else(|| std::env::temp_dir().join("rteams-meeting-assistant"));
        let dl = WhisperDownloader::new(data_dir);
        let tx = self.download_tx.clone();
        let tx2 = self.download_tx.clone();
        std::thread::spawn(move || {
            if let Err(e) = dl.ensure_downloaded(&tx) {
                let _ = tx2.send(format!("Download failed: {e}"));
            } else {
                let _ = tx2.send("Download complete! Set paths in Settings.".into());
            }
        });
    }

    fn run_diagnostics_full(&mut self) {
        if self.diagnostics_running {
            return;
        }
        let config = self.config.clone();
        let tx = self.diagnostics_tx.clone();
        std::thread::spawn(move || DiagnosticsRunner::run_full(config, tx));
    }

    fn run_diagnostic_one(&mut self, kind: DiagnosticKind) {
        if self.diagnostics_running {
            return;
        }
        let config = self.config.clone();
        let tx = self.diagnostics_tx.clone();
        std::thread::spawn(move || DiagnosticsRunner::run_one(kind, config, tx));
    }
}

impl eframe::App for MeetingAssistantApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(payload) = self.payload_rx.try_recv() {
            self.current_transcript = payload.source_text.clone();
            self.current_translation = payload.translated_text;
            self.current_suggestions = payload.suggestions;
            let entry = format!("[{}] {}", payload.speaker_label, payload.source_text);
            self.transcript_history.push(entry);
            if self.transcript_history.len() > 50 {
                self.transcript_history.remove(0);
            }
        }

        while let Ok(summary) = self.summary_rx.try_recv() {
            self.summary_text = summary;
            self.summary_generating = false;
            self.status_message = "Summary ready".to_string();
        }

        while let Ok(msg) = self.download_rx.try_recv() {
            self.is_downloading = false;
            self.status_message = msg.clone();
            if msg.starts_with("Download complete") {
                let data_dir =
                    directories::ProjectDirs::from("com", "rteams", "RTeamsMeetingAssistant")
                        .map(|p| p.data_dir().to_path_buf())
                        .unwrap_or_else(|| std::env::temp_dir().join("rteams-meeting-assistant"));
                let dl = WhisperDownloader::new(data_dir);
                let bp = dl.bin_path().to_string_lossy().to_string();
                let mp = dl.model_path().to_string_lossy().to_string();
                if self.config.whisper_binary.is_empty() {
                    self.config.whisper_binary = bp;
                }
                if self.config.whisper_model.is_empty() {
                    self.config.whisper_model = mp;
                }
            }
        }

        while let Ok(event) = self.diagnostics_rx.try_recv() {
            match event {
                DiagnosticEvent::Started(kind) => {
                    self.diagnostics_running = true;
                    self.diagnostics.mark_running(kind);
                    self.status_message = format!("Testing {}...", kind.label());
                }
                DiagnosticEvent::Finished(result) => {
                    self.status_message =
                        format!("{}: {}", result.kind.label(), result.status.label());
                    self.diagnostics.apply(result);
                }
                DiagnosticEvent::Done => {
                    self.diagnostics_running = false;
                    self.status_message = "Diagnostics complete".to_string();
                }
            }
        }

        egui::TopBottomPanel::top("title_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("R Teams Meeting Assistant");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Settings").clicked() {
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
                                if let Some(speaker_end) = line.find(']') {
                                    let label = &line[..speaker_end + 1];
                                    let text = &line[speaker_end + 1..];
                                    ui.horizontal(|ui| {
                                        ui.colored_label(egui::Color32::LIGHT_BLUE, label);
                                        ui.label(text);
                                    });
                                } else {
                                    ui.label(line);
                                }
                            }
                        });
                });

                ui.separator();
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let tabs = [
                            ("Translate", RightTab::Translation),
                            ("Suggest", RightTab::Suggestions),
                            ("Notes", RightTab::Notes),
                        ];
                        for (label, tab) in tabs {
                            let selected = self.right_tab == tab;
                            if ui.selectable_label(selected, label).clicked() {
                                if tab == RightTab::Notes {
                                    self.saved_notes = list_notes(&self.config.notes_dir);
                                }
                                self.right_tab = tab;
                            }
                        }
                    });

                    ui.separator();
                    match self.right_tab {
                        RightTab::Translation => {
                            egui::ScrollArea::vertical()
                                .id_salt("translation")
                                .max_height(avail.y - 200.0)
                                .show(ui, |ui| {
                                    ui.label(&self.current_translation);
                                });
                        }
                        RightTab::Suggestions => {
                            suggestions_tab(ui, &self.current_suggestions, ctx);
                        }
                        RightTab::Notes => {
                            notes_tab(ui, self);
                        }
                    }
                });
            });
        });

        if self.is_recording && self.start_time.is_some() {
            if self.last_auto_save.elapsed() >= std::time::Duration::from_secs(300) {
                if !self.transcript_history.is_empty() {
                    if let Ok(path) =
                        save_transcript(&self.transcript_history, &self.config.notes_dir)
                    {
                        self.saved_notes.push(path.clone());
                        self.status_message = format!(
                            "Auto-saved: {}",
                            path.file_name().unwrap().to_string_lossy()
                        );
                    }
                }
                self.last_auto_save = std::time::Instant::now();
            }
        }

        egui::TopBottomPanel::bottom("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if self.is_recording {
                    if let Some(start) = self.start_time {
                        let elapsed = start.elapsed();
                        let mins = elapsed.as_secs() / 60;
                        let secs = elapsed.as_secs() % 60;
                        ui.label(format!("{:02}:{:02}", mins, secs));
                    }
                }

                let btn_label = if self.is_recording { "Stop" } else { "Start" };
                if ui.button(btn_label).clicked() {
                    if self.is_recording {
                        self.stop_pipeline();
                    } else {
                        self.start_pipeline();
                    }
                }

                if ui.button("Save Transcript").clicked() {
                    if !self.transcript_history.is_empty() {
                        if let Ok(path) =
                            save_transcript(&self.transcript_history, &self.config.notes_dir)
                        {
                            self.saved_notes.push(path.clone());
                            self.status_message =
                                format!("Saved: {}", path.file_name().unwrap().to_string_lossy());
                        }
                    }
                }

                if self.config.whisper_binary.is_empty() && !self.is_downloading {
                    if ui.button("Download Whisper").clicked() {
                        self.start_download();
                    }
                }

                if self.is_downloading {
                    ui.label("Downloading...");
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(&self.status_message);
                });
            });
        });

        ctx.request_repaint();
    }
}

fn suggestions_tab(ui: &mut egui::Ui, suggestions: &[String], ctx: &egui::Context) {
    ui.label(egui::RichText::new("REPLY SUGGESTIONS").strong().size(14.0));
    if suggestions.is_empty() {
        ui.label("(waiting for input...)");
        return;
    }
    egui::ScrollArea::vertical()
        .id_salt("suggestions")
        .max_height(ui.available_height() - 10.0)
        .show(ui, |ui| {
            for s in suggestions {
                ui.horizontal(|ui| {
                    if ui.button("Copy").clicked() {
                        ctx.copy_text(s.clone());
                    }
                    ui.label(s);
                });
            }
        });
}

fn notes_tab(ui: &mut egui::Ui, app: &mut MeetingAssistantApp) {
    ui.label(egui::RichText::new("SAVED NOTES").strong().size(14.0));
    ui.horizontal(|ui| {
        if ui.button("Refresh").clicked() {
            app.saved_notes = list_notes(&app.config.notes_dir);
        }
        if ui.button("Open Folder").clicked() {
            let dir = &app.config.notes_dir;
            let _ = std::process::Command::new("explorer").arg(dir).spawn();
        }
    });

    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt("notes-list")
        .max_height(ui.available_height() * 0.4)
        .show(ui, |ui| {
            if app.saved_notes.is_empty() {
                ui.label("(no notes saved yet)");
            }
            for path in &app.saved_notes {
                ui.horizontal(|ui| {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if ui.button("Open").clicked() {
                        let _ = std::process::Command::new("cmd")
                            .args(["/c", "start", "", &path.to_string_lossy()])
                            .spawn();
                    }
                    ui.label(name);
                });
            }
        });

    ui.separator();
    ui.label(egui::RichText::new("SUMMARY").strong().size(14.0));
    if app.summary_generating {
        ui.label("Generating...");
    } else if ui.button("Generate Summary").clicked() {
        if app.transcript_history.is_empty() {
            app.status_message = "No transcript to summarize".to_string();
        } else {
            app.summary_generating = true;
            app.summary_text.clear();
            app.status_message = "Generating summary...".to_string();
            let th = app.transcript_history.clone();
            let ep = app.config.ollama_endpoint.clone();
            let model = app.config.suggester_model.clone();
            let tx = app.summary_tx.clone();
            std::thread::spawn(move || {
                spawn_summarize(th, &ep, &model, &tx);
            });
        }
    }
    if !app.summary_text.is_empty() {
        egui::ScrollArea::vertical()
            .id_salt("summary")
            .max_height(ui.available_height() - 10.0)
            .show(ui, |ui| {
                ui.label(&app.summary_text);
            });
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

        ui.separator();
        ui.heading("Audio");
        ui.horizontal(|ui| {
            ui.label("Audio Input:");
            let selected = if self.config.audio_input_device.is_empty() {
                "Default".to_string()
            } else {
                self.config.audio_input_device.clone()
            };
            egui::ComboBox::from_id_salt("audio-input-device")
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.config.audio_input_device,
                        String::new(),
                        "Default",
                    );
                    for name in &self.audio_input_devices {
                        ui.selectable_value(
                            &mut self.config.audio_input_device,
                            name.clone(),
                            name,
                        );
                    }
                });
            if ui.button("Refresh Audio Devices").clicked() {
                self.audio_input_devices = AudioCapture::input_device_names();
            }
        });
        ui.checkbox(&mut self.config.capture_system_audio, "Capture System Audio");

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

        self.diagnostics_panel(ui);

        if ui.button("Save & Back").clicked() {
            self.config.save();
            self.show_config = false;
        }

        ui.separator();
        ui.colored_label(egui::Color32::GRAY, "Config saved to:");
        ui.label(crate::config::Config::config_path().display().to_string());
    }

    fn diagnostics_panel(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.heading("Setup Diagnostics");
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!self.diagnostics_running, egui::Button::new("Test setup"))
                .clicked()
            {
                self.run_diagnostics_full();
            }
            if ui
                .add_enabled(!self.diagnostics_running, egui::Button::new("Test Mic"))
                .clicked()
            {
                self.run_diagnostic_one(DiagnosticKind::Mic);
            }
            if ui
                .add_enabled(
                    !self.diagnostics_running,
                    egui::Button::new("Test System Audio"),
                )
                .clicked()
            {
                self.run_diagnostic_one(DiagnosticKind::SystemAudio);
            }
            if ui
                .add_enabled(!self.diagnostics_running, egui::Button::new("Test Whisper"))
                .clicked()
            {
                self.run_diagnostic_one(DiagnosticKind::Whisper);
            }
            if ui
                .add_enabled(!self.diagnostics_running, egui::Button::new("Test Ollama"))
                .clicked()
            {
                self.run_diagnostic_one(DiagnosticKind::Ollama);
            }
        });

        ui.separator();
        for kind in [
            DiagnosticKind::Mic,
            DiagnosticKind::SystemAudio,
            DiagnosticKind::Whisper,
            DiagnosticKind::Ollama,
        ] {
            if let Some(result) = self.diagnostics.results.get(&kind) {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(kind.label()).strong());
                    ui.colored_label(status_color(result.status), result.status.label());
                    ui.label(&result.message);
                });
                if !result.hint.is_empty() {
                    ui.colored_label(egui::Color32::YELLOW, &result.hint);
                }
            }
        }

        ui.horizontal(|ui| {
            if ui.button("Copy diagnostics").clicked() {
                ui.ctx()
                    .copy_text(self.diagnostics.format_for_clipboard(&self.config));
                self.status_message = "Diagnostics copied".to_string();
            }
        });

        ui.label(egui::RichText::new("Diagnostics log").strong());
        egui::ScrollArea::vertical()
            .id_salt("diagnostics-log")
            .max_height(160.0)
            .show(ui, |ui| {
                ui.monospace(&self.diagnostics.log);
            });
    }
}

fn status_color(status: DiagnosticStatus) -> egui::Color32 {
    match status {
        DiagnosticStatus::Ok => egui::Color32::GREEN,
        DiagnosticStatus::Warning => egui::Color32::YELLOW,
        DiagnosticStatus::Failed => egui::Color32::RED,
        DiagnosticStatus::Running => egui::Color32::LIGHT_BLUE,
        DiagnosticStatus::NotRun => egui::Color32::GRAY,
    }
}
