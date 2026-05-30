//! Application configuration types
use serde::{Deserialize, Serialize};

pub use crate::ui::WindowSettings;
pub use crate::meeting::config::MeetingNotesConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub window_settings: WindowSettings,
    pub profiles: Vec<Profile>,
    pub current_profile_id: Option<String>,
    #[serde(default = "default_memory_config")]
    pub memory_optimization: MemoryOptimization,
    #[serde(default = "default_meeting_notes_config")]
    pub meeting_notes: MeetingNotesConfig,
}

fn default_memory_config() -> MemoryOptimization {
    MemoryOptimization::default()
}

fn default_meeting_notes_config() -> MeetingNotesConfig {
    MeetingNotesConfig::default()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOptimization {
    pub enabled: bool,
    pub max_cache_size_mb: u32,
    pub disable_gpu: bool,
    pub disable_animations: bool,
    pub idle_timeout_secs: u32,
}

impl Default for MemoryOptimization {
    fn default() -> Self {
        Self {
            enabled: true,
            max_cache_size_mb: 10,
            disable_gpu: true,
            disable_animations: true,
            idle_timeout_secs: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub teams_url: String,
    pub is_default: bool,
}
