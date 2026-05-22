//! Configuration management and storage
use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::AppConfig;

#[derive(Debug)]
pub struct ConfigManager {
    config_path: PathBuf,
    default_config: AppConfig,
}

impl ConfigManager {
    pub fn new() -> Self {
        let proj_dirs = ProjectDirs::from("com", "thuong", "rust_teams")
            .expect("Failed to get project directories");
        let config_path = proj_dirs.config_dir().join("config.json");

        Self {
            config_path,
            default_config: AppConfig {
                window_settings: crate::WindowSettings {
                    width: 1200,
                    height: 800,
                    x: None,
                    y: None,
                    maximized: false,
                    always_on_top: false,
                    transparent: false,
                },
                profiles: vec![],
                current_profile_id: None,
            },
        }
    }

    pub fn load(&self) -> Result<AppConfig> {
        if !self.config_path.exists() {
            eprintln!("Config not found, creating default → {}", self.config_path.display());
            self.save_defaults()?;
            return Ok(self.default_config.clone());
        }

        let content = fs::read_to_string(&self.config_path)?;
        let cfg: AppConfig = serde_json::from_str(&content)?;
        Ok(cfg)
    }

    pub fn save(&self, config: &AppConfig) -> Result<()> {
        fs::create_dir_all(self.config_path.parent().unwrap())?;
        let content = serde_json::to_string_pretty(config)?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }

    pub fn save_defaults(&self) -> Result<()> {
        if !self.config_path.exists() {
            self.save(&self.default_config)?;
        }
        Ok(())
    }

    pub fn validate(&self, _config: &AppConfig) -> Result<()> {
        // TODO: Add validation logic
        Ok(())
    }
}