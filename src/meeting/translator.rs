//! Translation providers
//! Supports OpenAI Chat Completions (gpt-4o-mini), Google Translate v2,
//! DeepL Free/Pro, and local Ollama.

#![allow(dead_code)]

use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

use super::realtime_config::TranslateConfig;

/// Translator trait
#[async_trait]
pub trait Translator: Send + Sync {
    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String>;
}

/// Create translator from config
pub fn create_translator(config: &TranslateConfig) -> Box<dyn Translator> {
    match config.provider_type.as_str() {
        "openai" => Box::new(OpenAiTranslator::new(
            &config.api_url,
            &config.api_key,
        )),
        "ollama" => Box::new(OllamaTranslator::new(
            &config.api_url,
            &config.extra,
        )),
        "google" => Box::new(GoogleTranslator::new(
            &config.api_url,
            &config.api_key,
        )),
        "deepl" => Box::new(DeepLTranslator::new(
            &config.api_url,
            &config.api_key,
        )),
        other => {
            log::warn!(
                "Unknown translator '{}', falling back to OpenAI",
                other
            );
            Box::new(OpenAiTranslator::new(
                &config.api_url,
                &config.api_key,
            ))
        }
    }
}

// ---------------- OpenAI ----------------

pub struct OpenAiTranslator {
    client: Client,
    api_url: String,
    api_key: String,
}

impl OpenAiTranslator {
    pub fn new(api_url: &str, api_key: &str) -> Self {
        Self {
            client: Client::new(),
            api_url: api_url.to_string(),
            api_key: api_key.to_string(),
        }
    }
}

#[async_trait]
impl Translator for OpenAiTranslator {
    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String> {
        let prompt = format!(
            "Translate the following text from {} to {}. \
             Preserve the tone, register, and any named entities. \
             Output only the translation, no commentary.\n\nText: {}",
            source_lang, target_lang, text
        );

        let body = serde_json::json!({
            "model": "gpt-4o-mini",
            "messages": [
                {"role": "system", "content": "You are a professional simultaneous interpreter."},
                {"role": "user",   "content": prompt}
            ],
            "temperature": 0.2,
            "max_tokens": 1024
        });

        let url = format!("{}/chat/completions", self.api_url);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI translate error: {}", err);
        }

        let v: serde_json::Value = resp.json().await?;
        let out = v["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();
        Ok(out)
    }
}

// ---------------- Ollama (local) ----------------

pub struct OllamaTranslator {
    client: Client,
    api_url: String,
    model: String,
}

impl OllamaTranslator {
    pub fn new(api_url: &str, model: &str) -> Self {
        let model = if model.is_empty() {
            "llama3".to_string()
        } else {
            model.to_string()
        };
        Self {
            client: Client::new(),
            api_url: api_url.to_string(),
            model,
        }
    }
}

#[async_trait]
impl Translator for OllamaTranslator {
    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String> {
        let prompt = format!(
            "Translate the following text from {} to {}. Output only the translation:\n\n{}",
            source_lang, target_lang, text
        );
        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "options": { "temperature": 0.1, "num_predict": 1024 }
        });
        let url = format!("{}/api/generate", self.api_url);
        let resp = self.client.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("Ollama translate error: {}", err);
        }
        let v: serde_json::Value = resp.json().await?;
        Ok(v["response"].as_str().unwrap_or("").trim().to_string())
    }
}

// ---------------- Google Translate v2 ----------------

pub struct GoogleTranslator {
    client: Client,
    api_url: String,
    api_key: String,
}

impl GoogleTranslator {
    pub fn new(api_url: &str, api_key: &str) -> Self {
        // api_url can override the default Google endpoint
        let api_url = if api_url.is_empty() {
            "https://translation.googleapis.com/language/translate/v2".to_string()
        } else {
            api_url.to_string()
        };
        Self {
            client: Client::new(),
            api_url,
            api_key: api_key.to_string(),
        }
    }
}

#[async_trait]
impl Translator for GoogleTranslator {
    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String> {
        let body = serde_json::json!({
            "q": text,
            "source": source_lang,
            "target": target_lang,
            "format": "text",
            "key": self.api_key
        });
        let resp = self
            .client
            .post(&self.api_url)
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("Google translate error: {}", err);
        }
        let v: serde_json::Value = resp.json().await?;
        let translated = v["data"]["translations"][0]["translatedText"]
            .as_str()
            .unwrap_or("")
            .to_string();
        Ok(translated)
    }
}

// ---------------- DeepL ----------------

pub struct DeepLTranslator {
    client: Client,
    api_url: String,
    api_key: String,
}

impl DeepLTranslator {
    pub fn new(api_url: &str, api_key: &str) -> Self {
        // Default to free API endpoint if user leaves it blank
        let api_url = if api_url.is_empty() {
            "https://api-free.deepl.com/v2/translate".to_string()
        } else {
            api_url.to_string()
        };
        Self {
            client: Client::new(),
            api_url,
            api_key: api_key.to_string(),
        }
    }
}

#[async_trait]
impl Translator for DeepLTranslator {
    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String> {
        // DeepL expects upper-case codes (e.g. EN, VI) optionally with regional variants
        let source = source_lang.to_uppercase();
        let target = target_lang.to_uppercase();

        let form = reqwest::multipart::Form::new()
            .text("text", text.to_string())
            .text("source_lang", source)
            .text("target_lang", target);

        let resp = self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("DeepL-Auth-Key {}", self.api_key))
            .multipart(form)
            .send()
            .await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("DeepL translate error: {}", err);
        }
        let v: serde_json::Value = resp.json().await?;
        let translated = v["translations"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        Ok(translated)
    }
}
