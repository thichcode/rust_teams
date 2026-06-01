//! Suggestion engine: produces N short reply candidates the user can say next
//! given the rolling conversation context and the latest transcript line.

#![allow(dead_code)]

use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

use super::realtime_config::SuggestionConfig;

#[async_trait]
pub trait Suggester: Send + Sync {
    /// Produce N reply suggestions.
    /// `context` contains the previous transcript lines (older -> newer).
    /// `latest` is the most recent transcript line.
    /// `lang` is the language the suggestions should be written in.
    async fn suggest(
        &self,
        context: &str,
        latest: &str,
        lang: &str,
        n: usize,
    ) -> Result<Vec<String>>;
}

pub fn create_suggester(config: &SuggestionConfig) -> Box<dyn Suggester> {
    match config.provider_type.as_str() {
        "openai" => Box::new(OpenAiSuggester::new(
            &config.api_url,
            &config.api_key,
            &config.model,
            &config.system_prompt,
        )),
        "ollama" => Box::new(OllamaSuggester::new(
            &config.api_url,
            &config.model,
            &config.system_prompt,
        )),
        other => {
            log::warn!(
                "Unknown suggester '{}', falling back to OpenAI",
                other
            );
            Box::new(OpenAiSuggester::new(
                &config.api_url,
                &config.api_key,
                &config.model,
                &config.system_prompt,
            ))
        }
    }
}

// ---------------- OpenAI ----------------

pub struct OpenAiSuggester {
    client: Client,
    api_url: String,
    api_key: String,
    model: String,
    system_prompt: String,
}

impl OpenAiSuggester {
    pub fn new(
        api_url: &str,
        api_key: &str,
        model: &str,
        system_prompt: &str,
    ) -> Self {
        Self {
            client: Client::new(),
            api_url: api_url.to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            system_prompt: system_prompt.to_string(),
        }
    }
}

#[async_trait]
impl Suggester for OpenAiSuggester {
    async fn suggest(
        &self,
        context: &str,
        latest: &str,
        lang: &str,
        n: usize,
    ) -> Result<Vec<String>> {
        let sys = self
            .system_prompt
            .replace("{n}", &n.to_string())
            .replace("{lang}", lang);

        let user = if context.trim().is_empty() {
            format!(
                "The other person just said: \"{}\". \
                 Suggest {} short replies in {}.",
                latest, n, lang
            )
        } else {
            format!(
                "Recent conversation:\n{}\n\nThe other person just said: \"{}\".\n\n\
                 Suggest {} short replies in {} that I could say next.",
                context, latest, n, lang
            )
        };

        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": sys},
                {"role": "user",   "content": user}
            ],
            "temperature": 0.7,
            "max_tokens": 512
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
            anyhow::bail!("OpenAI suggest error: {}", err);
        }

        let v: serde_json::Value = resp.json().await?;
        let raw = v["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(parse_suggestions(&raw, n))
    }
}

// ---------------- Ollama ----------------

pub struct OllamaSuggester {
    client: Client,
    api_url: String,
    model: String,
    system_prompt: String,
}

impl OllamaSuggester {
    pub fn new(api_url: &str, model: &str, system_prompt: &str) -> Self {
        Self {
            client: Client::new(),
            api_url: api_url.to_string(),
            model: model.to_string(),
            system_prompt: system_prompt.to_string(),
        }
    }
}

#[async_trait]
impl Suggester for OllamaSuggester {
    async fn suggest(
        &self,
        context: &str,
        latest: &str,
        lang: &str,
        n: usize,
    ) -> Result<Vec<String>> {
        let sys = self
            .system_prompt
            .replace("{n}", &n.to_string())
            .replace("{lang}", lang);
        let prompt = format!(
            "{}\n\nRecent conversation:\n{}\n\nLatest: \"{}\"\n\n\
             Reply with a JSON array of {} short replies in {}. No commentary.",
            sys, context, latest, n, lang
        );

        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "options": { "temperature": 0.7, "num_predict": 512 }
        });
        let url = format!("{}/api/generate", self.api_url);
        let resp = self.client.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("Ollama suggest error: {}", err);
        }
        let v: serde_json::Value = resp.json().await?;
        let raw = v["response"].as_str().unwrap_or("").to_string();
        Ok(parse_suggestions(&raw, n))
    }
}

/// Parse suggestion output. The LLM is asked for a JSON array, but it may
/// also return numbered lists or plain lines. We try JSON first, then fall
/// back to bullet/number parsing, then to non-empty lines.
fn parse_suggestions(raw: &str, n: usize) -> Vec<String> {
    let trimmed = raw.trim();

    // 1) JSON array
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(arr) = v.as_array() {
            let out: Vec<String> = arr
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty())
                .take(n)
                .collect();
            if !out.is_empty() {
                return out;
            }
        }
    }

    // 2) JSON object with key like "suggestions"
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(arr) = v.get("suggestions").and_then(|x| x.as_array()) {
            let out: Vec<String> = arr
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty())
                .take(n)
                .collect();
            if !out.is_empty() {
                return out;
            }
        }
    }

    // 3) Markdown list / numbered
    let mut out: Vec<String> = Vec::new();
    for line in trimmed.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        let candidate = strip_prefixes(s);
        if !candidate.is_empty() && candidate.len() > 1 {
            out.push(candidate);
            if out.len() >= n {
                break;
            }
        }
    }
    if !out.is_empty() {
        return out;
    }

    // 4) Fallback: any non-empty line
    trimmed
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .take(n)
        .map(str::to_string)
        .collect()
}

fn strip_prefixes(s: &str) -> String {
    let prefixes = [
        "- ", "* ", "• ", "· ",
    ];
    for p in prefixes {
        if let Some(rest) = s.strip_prefix(p) {
            return rest.trim().to_string();
        }
    }
    // "1. " "12) "
    let mut chars = s.chars();
    let mut digits = String::new();
    while let Some(c) = chars.clone().next() {
        if c.is_ascii_digit() {
            digits.push(c);
            chars.next();
        } else {
            break;
        }
    }
    if !digits.is_empty() {
        let after = chars.as_str();
        if let Some(rest) = after.strip_prefix(". ") {
            return rest.trim().to_string();
        }
        if let Some(rest) = after.strip_prefix(") ") {
            return rest.trim().to_string();
        }
    }
    s.to_string()
}
