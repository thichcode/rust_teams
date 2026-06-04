//! Configuration management and storage
use anyhow::Result;
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

use crate::app::{AppConfig, MemoryOptimization, Profile, WindowSettings};
use crate::meeting::config::MeetingNotesConfig;
use crate::meeting::realtime_config::{LocalPreset, RealtimeTranslateConfig};

#[derive(Debug)]
pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let proj_dirs = ProjectDirs::from("com", "rust-teams", "app")
            .expect("Failed to get project directories");
        let config_path = proj_dirs.config_dir().join("config.json");

        Self { config_path }
    }

    pub fn default_config(&self) -> AppConfig {
        AppConfig {
            window_settings: WindowSettings {
                width: 1200,
                height: 800,
                x: None,
                y: None,
                maximized: false,
            },
            profiles: vec![Profile {
                id: "default".to_string(),
                name: "Microsoft Teams".to_string(),
                teams_url: "https://teams.microsoft.com".to_string(),
                is_default: true,
            }],
            current_profile_id: Some("default".to_string()),
            memory_optimization: MemoryOptimization::default(),
            meeting_notes: MeetingNotesConfig::default(),
            realtime_translate: RealtimeTranslateConfig::default(),
        }
    }

    pub fn load(&self) -> Result<AppConfig> {
        if !self.config_path.exists() {
            log::info!(
                "Config not found, creating default → {}",
                self.config_path.display()
            );
            let default = self.default_config();
            self.save(&default)?;
            return Ok(default);
        }

        let content = fs::read_to_string(&self.config_path)?;
        let cfg: AppConfig = serde_json::from_str(&content)?;
        Ok(cfg)
    }

    pub fn save(&self, config: &AppConfig) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(config)?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }

    /// Update API keys for realtime translate (stt, translator, suggester).
    /// Returns the updated `RealtimeTranslateConfig` so callers can apply
    /// it to the running pipeline without restart.
    pub fn update_api_keys(
        &self,
        stt_key: Option<String>,
        translator_key: Option<String>,
        suggester_key: Option<String>,
    ) -> Result<RealtimeTranslateConfig> {
        let mut cfg = self.load().unwrap_or_else(|_| self.default_config());
        if let Some(k) = stt_key {
            cfg.realtime_translate.stt.api_key = k;
        }
        if let Some(k) = translator_key {
            cfg.realtime_translate.translator.api_key = k;
        }
        if let Some(k) = suggester_key {
            cfg.realtime_translate.suggester.api_key = k;
        }
        self.save(&cfg)?;
        Ok(cfg.realtime_translate)
    }

    /// Public accessor for the on-disk config path (for diagnostics / messages)
    pub fn config_path(&self) -> &std::path::Path {
        &self.config_path
    }

    /// Update the local-mode preset in `realtime_translate`, persist to
    /// disk, and return the updated config. Caller is expected to apply
    /// the returned config to the running pipeline before returning.
    #[allow(dead_code)]
    pub fn update_local_preset(&self, preset: &LocalPreset) -> Result<RealtimeTranslateConfig> {
        let mut cfg = self.load().unwrap_or_else(|_| self.default_config());
        cfg.realtime_translate.apply_local_preset(preset);
        self.save(&cfg)?;
        Ok(cfg.realtime_translate)
    }

    #[allow(dead_code)]
    pub fn validate(&self, _config: &AppConfig) -> Result<()> {
        // TODO: Add validation logic
        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
