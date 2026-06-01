//! Speech-to-Text providers
//! Supports OpenAI Whisper API and local Whisper

#![allow(dead_code)]

use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

use super::config::SttConfig;

/// STT provider trait
#[async_trait]
pub trait SttProvider: Send + Sync {
    /// Transcribe audio to text
    async fn transcribe(&self, audio: &[f32], language: &str) -> Result<String>;
}

/// Create STT provider based on config
pub fn create_stt_provider(config: &SttConfig) -> Box<dyn SttProvider> {
    match config.provider_type.as_str() {
        "openai" => Box::new(OpenAiWhisper::new(
            &config.api_url,
            &config.api_key,
            &config.model,
        )),
        "local" | "whisper-cpp" | "whisper.cpp" => Box::new(LocalWhisper::new(
            &config.api_url,
            &config.api_key,
        )),
        _ => {
            log::warn!("Unknown STT provider '{}', falling back to OpenAI", config.provider_type);
            Box::new(OpenAiWhisper::new(
                &config.api_url,
                &config.api_key,
                &config.model,
            ))
        }
    }
}

/// OpenAI Whisper API provider
pub struct OpenAiWhisper {
    client: Client,
    api_url: String,
    api_key: String,
    model: String,
}

impl OpenAiWhisper {
    pub fn new(api_url: &str, api_key: &str, model: &str) -> Self {
        Self {
            client: Client::new(),
            api_url: api_url.to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }
}

#[async_trait]
impl SttProvider for OpenAiWhisper {
    async fn transcribe(&self, audio: &[f32], language: &str) -> Result<String> {
        // Convert audio to WAV
        let wav_data = super::audio::AudioCapture::to_wav(audio, 16000, 1)?;

        // Create multipart form
        let part = reqwest::multipart::Part::bytes(wav_data)
            .file_name("audio.wav")
            .mime_str("audio/wav")?;

        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", self.model.clone())
            .text("language", language.to_string())
            .text("response_format", "json".to_string());

        // Send request
        let url = format!("{}/audio/transcriptions", self.api_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Whisper API error: {}", error_text);
        }

        let result: serde_json::Value = response.json().await?;
        let text = result["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(text)
    }
}

/// Local Whisper provider using whisper.cpp CLI as subprocess.
///
/// The user must download a whisper.cpp release binary + a model file.
///   - Config `api_url` → path to `main.exe` (e.g. `C:\\whisper\\main.exe`)
///   - Config `api_key` → path to ggml model (e.g. `C:\\whisper\\models\\ggml-base.en.bin`)
pub struct LocalWhisper {
    whisper_bin: String,
    model_path: String,
}

impl LocalWhisper {
    pub fn new(whisper_bin: &str, model_path: &str) -> Self {
        Self {
            whisper_bin: whisper_bin.to_string(),
            model_path: model_path.to_string(),
        }
    }
}

#[async_trait]
impl SttProvider for LocalWhisper {
    async fn transcribe(&self, audio: &[f32], language: &str) -> Result<String> {
        let wav = super::audio::AudioCapture::to_wav(audio, 16000, 1)?;
        let tmp_dir = std::env::temp_dir();
        let tmp_wav = tmp_dir.join(format!("whisper_{}.wav", uuid::Uuid::new_v4()));
        let tmp_out = tmp_wav.with_extension("txt");
        std::fs::write(&tmp_wav, &wav)?;

        // Build command: main.exe -m <model> -f <input.wav> -otxt -l <lang>
        let lang_flag = if language.is_empty() { "auto" } else { language };
        let output = tokio::process::Command::new(&self.whisper_bin)
            .arg("-m")
            .arg(&self.model_path)
            .arg("-f")
            .arg(tmp_wav.as_os_str())
            .arg("-otxt")
            .arg("-l")
            .arg(lang_flag)
            .arg("--no-prints")
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to run whisper: {}", e))?;

        // Cleanup temp wav
        let _ = std::fs::remove_file(&tmp_wav);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("whisper exited with {}: {}", output.status, stderr);
        }

        // Read output txt file
        let text = tokio::fs::read_to_string(&tmp_out)
            .await
            .unwrap_or_else(|_| String::from_utf8_lossy(&output.stdout).to_string());
        let _ = std::fs::remove_file(&tmp_out);

        Ok(text.trim().to_string())
    }
}
