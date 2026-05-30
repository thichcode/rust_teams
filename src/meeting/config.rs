//! Meeting notes configuration

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main meeting notes configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingNotesConfig {
    /// Enable meeting notes feature
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Auto-start recording when meeting detected
    #[serde(default = "default_auto_start")]
    pub auto_start: bool,

    /// Output directory for notes
    #[serde(default = "default_output_dir")]
    pub output_dir: String,

    /// Supported languages
    #[serde(default = "default_languages")]
    pub languages: Vec<String>,

    /// Audio configuration
    pub audio: AudioConfig,

    /// STT provider configuration
    pub stt_provider: SttConfig,

    /// LLM provider configuration
    pub llm_provider: LlmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    /// Record system audio (speakers)
    #[serde(default = "default_true")]
    pub record_system_audio: bool,

    /// Record microphone
    #[serde(default = "default_true")]
    pub record_microphone: bool,

    /// Sample rate (Hz)
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,

    /// Number of channels
    #[serde(default = "default_channels")]
    pub channels: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    /// Provider type: "openai" or "local"
    #[serde(default = "default_stt_type")]
    pub provider_type: String,

    /// API URL
    #[serde(default = "default_stt_api_url")]
    pub api_url: String,

    /// API key (for OpenAI)
    #[serde(default)]
    pub api_key: String,

    /// Model name
    #[serde(default = "default_stt_model")]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Provider type: "ollama" or "openai"
    #[serde(default = "default_llm_type")]
    pub provider_type: String,

    /// API URL
    #[serde(default = "default_llm_api_url")]
    pub api_url: String,

    /// API key (for OpenAI)
    #[serde(default)]
    pub api_key: String,

    /// Model name
    #[serde(default = "default_llm_model")]
    pub model: String,
}

// Default value functions
fn default_enabled() -> bool { true }
fn default_auto_start() -> bool { true }
fn default_output_dir() -> String { 
    dirs().join("Documents").join("RustTeams").join("Notes").to_string_lossy().to_string()
}
fn default_languages() -> Vec<String> { vec!["en".to_string(), "vi".to_string()] }
fn default_true() -> bool { true }
fn default_sample_rate() -> u32 { 16000 }
fn default_channels() -> u16 { 1 }
fn default_stt_type() -> String { "openai".to_string() }
fn default_stt_api_url() -> String { "https://api.openai.com/v1".to_string() }
fn default_stt_model() -> String { "whisper-1".to_string() }
fn default_llm_type() -> String { "ollama".to_string() }
fn default_llm_api_url() -> String { "http://localhost:11434".to_string() }
fn default_llm_model() -> String { "llama3".to_string() }

fn dirs() -> PathBuf {
    directories::ProjectDirs::from("com", "rust-teams", "app")
        .map(|p| p.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

impl Default for MeetingNotesConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            auto_start: default_auto_start(),
            output_dir: default_output_dir(),
            languages: default_languages(),
            audio: AudioConfig::default(),
            stt_provider: SttConfig::default(),
            llm_provider: LlmConfig::default(),
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            record_system_audio: default_true(),
            record_microphone: default_true(),
            sample_rate: default_sample_rate(),
            channels: default_channels(),
        }
    }
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            provider_type: default_stt_type(),
            api_url: default_stt_api_url(),
            api_key: String::new(),
            model: default_stt_model(),
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider_type: default_llm_type(),
            api_url: default_llm_api_url(),
            api_key: String::new(),
            model: default_llm_model(),
        }
    }
}
