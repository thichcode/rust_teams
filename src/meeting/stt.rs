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

/// Local Whisper provider (placeholder for future implementation)
pub struct LocalWhisper {
    #[allow(dead_code)]
    model_path: String,
}

impl LocalWhisper {
    #[allow(dead_code)]
    pub fn new(model_path: &str) -> Self {
        Self {
            model_path: model_path.to_string(),
        }
    }
}

#[async_trait]
#[allow(dead_code)]
impl SttProvider for LocalWhisper {
    async fn transcribe(&self, _audio: &[f32], _language: &str) -> Result<String> {
        anyhow::bail!("Local Whisper not yet implemented. Use OpenAI provider.")
    }
}
