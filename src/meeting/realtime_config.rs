//! Realtime translate configuration
//! Captures audio from Teams call, transcribes, translates, and suggests replies

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Realtime translate config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeTranslateConfig {
    /// Enable realtime translate feature
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Auto-start when call detected
    #[serde(default = "default_auto_start")]
    pub auto_start: bool,

    /// Source language code (e.g. "en")
    #[serde(default = "default_source_lang")]
    pub source_lang: String,

    /// Target language code (e.g. "vi")
    #[serde(default = "default_target_lang")]
    pub target_lang: String,

    /// Audio chunk duration in seconds
    #[serde(default = "default_chunk_secs")]
    pub chunk_duration_secs: u32,

    /// Show suggestion panel
    #[serde(default = "default_true")]
    pub show_suggestions: bool,

    /// Number of suggestions to show
    #[serde(default = "default_suggestion_count")]
    pub suggestion_count: u32,

    /// STT provider
    pub stt: SttRealtimeConfig,

    /// Translation provider
    pub translator: TranslateConfig,

    /// Suggestion LLM provider
    pub suggester: SuggestionConfig,

    /// User's local-mode wizard choices (persisted across restarts)
    #[serde(default)]
    pub local_preset: LocalPreset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttRealtimeConfig {
    #[serde(default = "default_stt_type")]
    pub provider_type: String,
    #[serde(default = "default_stt_api_url")]
    pub api_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_stt_model")]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateConfig {
    /// "openai", "google", "deepl", "ollama"
    #[serde(default = "default_translator_type")]
    pub provider_type: String,
    #[serde(default = "default_translator_api_url")]
    pub api_url: String,
    #[serde(default)]
    pub api_key: String,
    /// For Google: project id; DeepL: not needed
    #[serde(default)]
    pub extra: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionConfig {
    #[serde(default = "default_suggester_type")]
    pub provider_type: String,
    #[serde(default = "default_suggester_api_url")]
    pub api_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_suggester_model")]
    pub model: String,
    /// System prompt to bias suggestions (e.g. business context)
    #[serde(default = "default_suggester_system_prompt")]
    pub system_prompt: String,
}

fn default_enabled() -> bool { true }
fn default_auto_start() -> bool { true }
fn default_source_lang() -> String { "en".to_string() }
fn default_target_lang() -> String { "vi".to_string() }
fn default_chunk_secs() -> u32 { 5 }
fn default_true() -> bool { true }
fn default_suggestion_count() -> u32 { 3 }

fn default_stt_type() -> String { "openai".to_string() }
fn default_stt_api_url() -> String { "https://api.openai.com/v1".to_string() }
fn default_stt_model() -> String { "whisper-1".to_string() }

fn default_translator_type() -> String { "openai".to_string() }
fn default_translator_api_url() -> String { "https://api.openai.com/v1".to_string() }

fn default_suggester_type() -> String { "openai".to_string() }
fn default_suggester_api_url() -> String { "https://api.openai.com/v1".to_string() }
fn default_suggester_model() -> String { "gpt-4o-mini".to_string() }
fn default_suggester_system_prompt() -> String {
    "You are a helpful assistant in a business meeting. \
     Based on the conversation context, suggest {n} short, natural replies the user can say next. \
     Replies should be in {lang}, be polite and professional, and match the conversational tone. \
     Format as a JSON array of strings."
        .to_string()
}

/// Persisted user choices from the local-mode wizard.
/// Used by `RealtimeTranslateConfig::apply_local_preset` to switch all
/// 3 providers to local in one shot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalPreset {
    /// Whisper STT model id (e.g. "ggml-base.en")
    #[serde(default)]
    pub stt_model: String,
    /// Ollama model id for translation (e.g. "qwen2.5:7b")
    #[serde(default)]
    pub translator_model: String,
    /// Ollama model id for suggestions (e.g. "gemma3:4b")
    #[serde(default)]
    pub suggester_model: String,
    /// Ollama server URL, default http://localhost:11434
    #[serde(default = "default_ollama_endpoint")]
    pub ollama_endpoint: String,
    /// Absolute path to whisper.cpp `main.exe`
    #[serde(default)]
    pub whisper_binary: String,
    /// Absolute path to ggml-*.bin model file
    #[serde(default)]
    pub whisper_model: String,
    /// Unix timestamp of last successful readiness check
    #[serde(default)]
    pub last_checked: Option<i64>,
}

fn default_ollama_endpoint() -> String { "http://localhost:11434".to_string() }

impl Default for RealtimeTranslateConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            auto_start: default_auto_start(),
            source_lang: default_source_lang(),
            target_lang: default_target_lang(),
            chunk_duration_secs: default_chunk_secs(),
            show_suggestions: default_true(),
            suggestion_count: default_suggestion_count(),
            stt: SttRealtimeConfig::default(),
            translator: TranslateConfig::default(),
            suggester: SuggestionConfig::default(),
            local_preset: LocalPreset::default(),
        }
    }
}

impl Default for SttRealtimeConfig {
    fn default() -> Self {
        Self {
            provider_type: default_stt_type(),
            api_url: default_stt_api_url(),
            api_key: String::new(),
            model: default_stt_model(),
        }
    }
}

impl Default for TranslateConfig {
    fn default() -> Self {
        Self {
            provider_type: default_translator_type(),
            api_url: default_translator_api_url(),
            api_key: String::new(),
            extra: String::new(),
        }
    }
}

impl Default for SuggestionConfig {
    fn default() -> Self {
        Self {
            provider_type: default_suggester_type(),
            api_url: default_suggester_api_url(),
            api_key: String::new(),
            model: default_suggester_model(),
            system_prompt: default_suggester_system_prompt(),
        }
    }
}

impl Default for LocalPreset {
    fn default() -> Self {
        Self {
            stt_model: String::new(),
            translator_model: String::new(),
            suggester_model: String::new(),
            ollama_endpoint: default_ollama_endpoint(),
            whisper_binary: String::new(),
            whisper_model: String::new(),
            last_checked: None,
        }
    }
}

impl RealtimeTranslateConfig {
    /// Switch all 3 providers (stt, translator, suggester) to local,
    /// populating their `provider_type`, `api_url`, `model` fields
    /// from the given preset. Existing user settings (target_lang,
    /// suggestion_count, etc.) are preserved.
    pub fn apply_local_preset(&mut self, preset: &LocalPreset) {
        // STT → whisper.cpp local subprocess
        self.stt.provider_type = "local".to_string();
        self.stt.api_url = preset.whisper_binary.clone();
        self.stt.api_key = preset.whisper_model.clone();
        if !preset.stt_model.is_empty() {
            self.stt.model = preset.stt_model.clone();
        }
        // Translator → Ollama
        self.translator.provider_type = "ollama".to_string();
        self.translator.api_url = preset.ollama_endpoint.clone();
        if !preset.translator_model.is_empty() {
            self.translator.extra = preset.translator_model.clone();
        }
        // Suggester → Ollama
        self.suggester.provider_type = "ollama".to_string();
        self.suggester.api_url = preset.ollama_endpoint.clone();
        if !preset.suggester_model.is_empty() {
            self.suggester.model = preset.suggester_model.clone();
        }
        // Persist the preset so wizard can re-open pre-filled
        self.local_preset = preset.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_preset() -> LocalPreset {
        LocalPreset {
            stt_model: "ggml-base.en".into(),
            translator_model: "qwen2.5:7b".into(),
            suggester_model: "gemma3:4b".into(),
            ollama_endpoint: "http://localhost:11434".into(),
            whisper_binary: "C:/rteams/whisper/main.exe".into(),
            whisper_model: "C:/rteams/whisper/ggml-base.en.bin".into(),
            last_checked: Some(1718000000),
        }
    }

    #[test]
    fn apply_local_preset_swaps_all_three_providers() {
        let mut cfg = RealtimeTranslateConfig::default();
        assert_eq!(cfg.stt.provider_type, "openai");
        assert_eq!(cfg.translator.provider_type, "openai");
        assert_eq!(cfg.suggester.provider_type, "openai");

        cfg.apply_local_preset(&sample_preset());

        assert_eq!(cfg.stt.provider_type, "local");
        assert_eq!(cfg.stt.model, "ggml-base.en");
        assert_eq!(cfg.translator.provider_type, "ollama");
        assert_eq!(cfg.translator.api_url, "http://localhost:11434");
        assert_eq!(cfg.suggester.provider_type, "ollama");
        assert_eq!(cfg.suggester.model, "gemma3:4b");
    }

    #[test]
    fn apply_local_preset_preserves_other_settings() {
        let mut cfg = RealtimeTranslateConfig::default();
        cfg.target_lang = "ja".into();
        cfg.suggestion_count = 5;

        cfg.apply_local_preset(&sample_preset());

        assert_eq!(cfg.target_lang, "ja");
        assert_eq!(cfg.suggestion_count, 5);
    }

    #[test]
    fn default_local_preset_has_empty_models() {
        let p = LocalPreset::default();
        assert!(p.stt_model.is_empty());
        assert!(p.translator_model.is_empty());
        assert!(p.suggester_model.is_empty());
        assert_eq!(p.ollama_endpoint, "http://localhost:11434");
    }
}
