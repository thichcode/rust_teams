//! LLM providers for meeting summarization
//! Supports Ollama (local) and OpenAI GPT

#![allow(dead_code)]

use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

use super::config::LlmConfig;

/// LLM provider trait
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Summarize text using the LLM
    async fn summarize(&self, text: &str, prompt: &str) -> Result<String>;
}

/// Create LLM provider based on config
pub fn create_llm_provider(config: &LlmConfig) -> Box<dyn LlmProvider> {
    match config.provider_type.as_str() {
        "ollama" => Box::new(OllamaProvider::new(
            &config.api_url,
            &config.model,
        )),
        "openai" => Box::new(OpenAiProvider::new(
            &config.api_url,
            &config.api_key,
            &config.model,
        )),
        _ => {
            log::warn!("Unknown LLM provider '{}', falling back to Ollama", config.provider_type);
            Box::new(OllamaProvider::new(
                &config.api_url,
                &config.model,
            ))
        }
    }
}

/// Ollama provider (local)
pub struct OllamaProvider {
    client: Client,
    api_url: String,
    model: String,
}

impl OllamaProvider {
    pub fn new(api_url: &str, model: &str) -> Self {
        Self {
            client: Client::new(),
            api_url: api_url.to_string(),
            model: model.to_string(),
        }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn summarize(&self, text: &str, prompt: &str) -> Result<String> {
        let request_body = serde_json::json!({
            "model": self.model,
            "prompt": format!("{}\n\n{}", prompt, text),
            "stream": false,
            "options": {
                "temperature": 0.3,
                "num_predict": 2048
            }
        });

        let url = format!("{}/api/generate", self.api_url);
        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Ollama API error: {}", error_text);
        }

        let result: serde_json::Value = response.json().await?;
        let output = result["response"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(output)
    }
}

/// OpenAI GPT provider
pub struct OpenAiProvider {
    client: Client,
    api_url: String,
    api_key: String,
    model: String,
}

impl OpenAiProvider {
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
impl LlmProvider for OpenAiProvider {
    async fn summarize(&self, text: &str, prompt: &str) -> Result<String> {
        let request_body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": prompt},
                {"role": "user", "content": text}
            ],
            "temperature": 0.3,
            "max_tokens": 2048
        });

        let url = format!("{}/chat/completions", self.api_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI API error: {}", error_text);
        }

        let result: serde_json::Value = response.json().await?;
        let output = result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(output)
    }
}
