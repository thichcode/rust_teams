use anyhow::Result;
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

use crate::app::{AppConfig, LinuxBackend, MemoryOptimization, Profile, WebkitRenderMode, WindowSettings};

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
                teams_url: "https://teams.office.com".to_string(),
                is_default: true,
            }],
            current_profile_id: Some("default".to_string()),
            memory_optimization: MemoryOptimization::default(),
            browser_path: None,
            webkit_render_mode: WebkitRenderMode::default(),
            linux_backend: LinuxBackend::default(),
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

    #[allow(dead_code)]
    pub fn config_path(&self) -> &std::path::Path {
        &self.config_path
    }

    #[allow(dead_code)]
    pub fn validate(&self, _config: &AppConfig) -> Result<()> {
        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
