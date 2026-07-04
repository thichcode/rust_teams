use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Translator: Send + Sync {
    async fn translate(&self, text: &str, source_lang: &str, target_lang: &str) -> Result<String>;
}

pub struct OllamaTranslator {
    client: reqwest::Client,
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
            client: reqwest::Client::new(),
            api_url: api_url.to_string(),
            model,
        }
    }
}

#[async_trait]
impl Translator for OllamaTranslator {
    async fn translate(&self, text: &str, source_lang: &str, target_lang: &str) -> Result<String> {
        let prompt = format!(
            "Translate the following text from {source_lang} to {target_lang}. Output only the translation:\n\n{text}"
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
            anyhow::bail!("Ollama translate: {err}");
        }
        let v: serde_json::Value = resp.json().await?;
        Ok(v["response"].as_str().unwrap_or("").trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_trait() {
        let t = OllamaTranslator::new("http://localhost:11434", "llama3");
        assert_eq!(t.model, "llama3");
    }

    #[test]
    fn test_new_empty_model_uses_default() {
        let t = OllamaTranslator::new("http://localhost:11434", "");
        assert_eq!(t.model, "llama3");
    }

    #[test]
    fn test_trait_object() {
        let t = OllamaTranslator::new("http://localhost:11434", "llama3");
        let provider: &dyn Translator = &t;
        assert!(std::mem::size_of_val(provider) > 0);
    }
}
