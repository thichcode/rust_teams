use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Suggester: Send + Sync {
    async fn suggest(
        &self,
        context: &str,
        latest: &str,
        lang: &str,
        n: usize,
    ) -> Result<Vec<String>>;
}

pub struct OllamaSuggester {
    client: reqwest::Client,
    api_url: String,
    model: String,
}

impl OllamaSuggester {
    pub fn new(api_url: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_url: api_url.to_string(),
            model: model.to_string(),
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
        let prompt = format!(
            "You are a meeting assistant. Based on the conversation, suggest 3 short replies in {lang}: a professional explanation, a simple easy-to-understand explanation, and a quick clarification. \
             Format as a JSON array of strings.\n\nContext:\n{context}\n\nLatest: \"{latest}\""
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
            anyhow::bail!("Ollama suggest: {err}");
        }
        let v: serde_json::Value = resp.json().await?;
        let raw = v["response"].as_str().unwrap_or("").to_string();
        Ok(parse_suggestions(&raw, n))
    }
}

fn parse_suggestions(raw: &str, n: usize) -> Vec<String> {
    let trimmed = raw.trim();
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
    let mut out: Vec<String> = Vec::new();
    for line in trimmed.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        let prefix = s.trim_start_matches(|c: char| {
            c.is_ascii_digit() || c == '.' || c == ')' || c == '-' || c == '*' || c == '•'
        });
        let cleaned = if prefix != s { prefix.trim() } else { s };
        if !cleaned.is_empty() && cleaned.len() > 1 {
            out.push(cleaned.to_string());
            if out.len() >= n {
                break;
            }
        }
    }
    if !out.is_empty() {
        return out;
    }
    trimmed
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .take(n)
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_array() {
        let raw = r#"["Hello", "World", "Test"]"#;
        let result = parse_suggestions(raw, 3);
        assert_eq!(result, vec!["Hello", "World", "Test"]);
    }

    #[test]
    fn test_parse_json_with_suggestions_key() {
        let raw = r#"{"suggestions": ["A", "B", "C"]}"#;
        let result = parse_suggestions(raw, 3);
        assert_eq!(result, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_parse_numbered_list() {
        let raw = "1. First suggestion\n2. Second idea\n3. Third one";
        let result = parse_suggestions(raw, 2);
        assert_eq!(result, vec!["First suggestion", "Second idea"]);
    }

    #[test]
    fn test_parse_bullet_list() {
        let raw = "- Item one\n- Item two\n* Item three";
        let result = parse_suggestions(raw, 3);
        assert_eq!(result, vec!["Item one", "Item two", "Item three"]);
    }

    #[test]
    fn test_parse_limited_count() {
        let raw = "1. A\n2. B\n3. C";
        let result = parse_suggestions(raw, 2);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_parse_empty() {
        let result = parse_suggestions("", 3);
        assert!(result.is_empty());
    }
}
