//! Main application state and logic
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::config::ConfigManager;
use crate::ui::{WindowManager, WindowSettings};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
  pub window_settings: WindowSettings,
  pub profiles: Vec<Profile>,
  pub current_profile_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
  pub id: String,
  pub name: String,
  pub teams_url: String,
  pub is_default: bool,
}

pub struct App {
    pub config: AppConfig,
    config_manager: ConfigManager,
    window_manager: WindowManager,
}

impl App {
    pub fn new(config: AppConfig) -> Result<Self> {
        Ok(Self {
            config,
            config_manager: ConfigManager::new(),
            window_manager: WindowManager::new()?,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        // Create main window
        self.window_manager.create_window(&self.config.window_settings)?;
        self.window_manager.show()?;

        // Main event loop can go here
        Ok(())
    }
    
    pub fn init(&self) -> Result<()> { Ok(()) }
}