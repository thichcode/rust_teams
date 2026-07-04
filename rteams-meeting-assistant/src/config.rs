use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ollama_endpoint: String,
    pub whisper_binary: String,
    pub whisper_model: String,
    #[serde(default)]
    pub audio_input_device: String,
    #[serde(default = "default_capture_system_audio")]
    pub capture_system_audio: bool,
    pub source_lang: String,
    pub target_lang: String,
    pub translator_model: String,
    pub suggester_model: String,
    pub notes_dir: String,
    /// Global hotkey for toggle recording (e.g. "Ctrl+Space", "Alt+R").
    /// Set to empty string to disable.
    #[serde(default = "default_hotkey")]
    pub toggle_hotkey: String,
}

fn default_capture_system_audio() -> bool {
    true
}

fn default_hotkey() -> String {
    "Ctrl+Space".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ollama_endpoint: "http://localhost:11434".to_string(),
            whisper_binary: String::new(),
            whisper_model: String::new(),
            audio_input_device: String::new(),
            capture_system_audio: true,
            source_lang: "en".to_string(),
            target_lang: "vi".to_string(),
            translator_model: "qwen2.5:3b".to_string(),
            suggester_model: "qwen2.5:3b".to_string(),
            notes_dir: String::new(),
            toggle_hotkey: "Ctrl+Space".to_string(),
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

    pub fn data_dir() -> PathBuf {
        directories::ProjectDirs::from("com", "rteams", "RTeamsMeetingAssistant")
            .map(|p| p.data_dir().to_path_buf())
            .unwrap_or_else(|| std::env::temp_dir().join("rteams-meeting-assistant"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_config_defaults_to_default_mic_and_system_audio_enabled() {
        let json = r#"{
            "ollama_endpoint": "http://localhost:11434",
            "whisper_binary": "whisper.exe",
            "whisper_model": "model.bin",
            "source_lang": "en",
            "target_lang": "vi",
            "translator_model": "qwen2.5:3b",
            "suggester_model": "qwen2.5:3b",
            "notes_dir": "notes"
        }"#;

        let config: Config = serde_json::from_str(json).unwrap();

        assert_eq!(config.audio_input_device, "");
        assert!(config.capture_system_audio);
    }
}
