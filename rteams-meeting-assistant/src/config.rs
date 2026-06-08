use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ollama_endpoint: String,
    pub whisper_binary: String,
    pub whisper_model: String,
    pub source_lang: String,
    pub target_lang: String,
    pub translator_model: String,
    pub suggester_model: String,
    pub notes_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ollama_endpoint: "http://localhost:11434".to_string(),
            whisper_binary: String::new(),
            whisper_model: String::new(),
            source_lang: "en".to_string(),
            target_lang: "vi".to_string(),
            translator_model: "qwen2.5:7b".to_string(),
            suggester_model: "gemma3:4b".to_string(),
            notes_dir: String::new(),
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        directories::BaseDirs::new()
            .map(|d| d.config_dir().join("RTeamsMeetingAssistant"))
            .unwrap_or_else(|| PathBuf::from("."))
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let dir = Self::config_dir();
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::config_path(), json);
        }
    }
}
