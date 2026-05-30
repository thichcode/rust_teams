//! Meeting notes generator
//! Orchestrates audio capture, STT, and LLM to generate meeting notes

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use anyhow::Result;
use chrono::{Local, Utc};

use super::audio::AudioCapture;
use super::config::MeetingNotesConfig;
use super::llm::{self, LlmProvider};
use super::stt::{self, SttProvider};

/// Meeting notes data
#[derive(Debug, Clone)]
pub struct MeetingNotes {
    pub title: String,
    pub date: String,
    pub duration: String,
    pub transcript: String,
    pub summary: String,
    pub file_path: Option<PathBuf>,
}

/// Meeting notes generator
pub struct MeetingNotesGenerator {
    audio: AudioCapture,
    stt_provider: Box<dyn SttProvider>,
    llm_provider: Box<dyn LlmProvider>,
    config: MeetingNotesConfig,
    start_time: Option<std::time::Instant>,
    is_active: Arc<AtomicBool>,
}

impl MeetingNotesGenerator {
    /// Create new generator
    pub fn new(config: MeetingNotesConfig) -> Result<Self> {
        let audio = AudioCapture::new(config.audio.clone())?;
        let stt_provider = stt::create_stt_provider(&config.stt_provider);
        let llm_provider = llm::create_llm_provider(&config.llm_provider);

        Ok(Self {
            audio,
            stt_provider,
            llm_provider,
            config,
            start_time: None,
            is_active: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Start recording meeting
    pub fn start_meeting(&mut self) -> Result<()> {
        log::info!("Starting meeting recording...");
        self.audio.start_recording()?;
        self.start_time = Some(std::time::Instant::now());
        self.is_active.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// End recording and generate notes
    pub async fn end_meeting(&mut self) -> Result<MeetingNotes> {
        log::info!("Ending meeting recording...");

        // Stop recording
        let audio_samples = self.audio.stop_recording()?;
        self.is_active.store(false, Ordering::Relaxed);

        // Calculate duration
        let duration = self.start_time
            .map(|t| t.elapsed())
            .unwrap_or_default();
        let duration_str = format!("{}:{:02}", duration.as_secs() / 60, duration.as_secs() % 60);

        // Transcribe audio
        log::info!("Transcribing audio ({} samples)...", audio_samples.len());
        let language = self.config.languages.first().map(|s| s.as_str()).unwrap_or("en");
        let transcript = self.stt_provider.transcribe(&audio_samples, language).await?;

        // Generate summary
        log::info!("Generating meeting summary...");
        let prompt = self.get_summary_prompt();
        let summary = self.llm_provider.summarize(&transcript, &prompt).await?;

        // Create notes
        let notes = MeetingNotes {
            title: format!("Meeting {}", Local::now().format("%Y-%m-%d %H:%M")),
            date: Utc::now().format("%Y-%m-%d %H:%M UTC").to_string(),
            duration: duration_str,
            transcript,
            summary,
            file_path: None,
        };

        Ok(notes)
    }

    /// Save notes to file
    pub fn save_to_file(&self, notes: &MeetingNotes) -> Result<PathBuf> {
        let output_dir = PathBuf::from(&self.config.output_dir);
        std::fs::create_dir_all(&output_dir)?;

        let filename = format!(
            "meeting_{}.md",
            Utc::now().format("%Y%m%d_%H%M%S")
        );
        let file_path = output_dir.join(filename);

        let content = self.format_notes(notes);
        std::fs::write(&file_path, content)?;

        log::info!("Meeting notes saved to: {}", file_path.display());
        Ok(file_path)
    }

    /// Format notes as Markdown
    fn format_notes(&self, notes: &MeetingNotes) -> String {
        format!(
            r#"# {}

**Date:** {}
**Duration:** {}

---

## Summary

{}

---

## Transcript

{}
"#,
            notes.title, notes.date, notes.duration, notes.summary, notes.transcript
        )
    }

    /// Get prompt for meeting summarization
    fn get_summary_prompt(&self) -> String {
        r#"You are a meeting notes assistant. Summarize the following meeting transcript.

Please provide:
1. **Meeting Summary** (2-3 paragraphs)
2. **Key Discussion Points** (bullet list)
3. **Action Items** (who does what by when)
4. **Decisions Made** (list)
5. **Next Steps** (list)

Format in clean Markdown."#
            .to_string()
    }

    /// Check if recording is active
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Relaxed)
    }

    /// Get buffer length
    pub fn buffer_len(&self) -> usize {
        self.audio.buffer_len()
    }
}
