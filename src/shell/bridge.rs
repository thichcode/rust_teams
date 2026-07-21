use serde::{Deserialize, Serialize};

/// Shell commands sent from Dioxus UI to native event loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShellCommand {
    ReloadTeams,
    SwitchProfile(String),
    OpenSettings,
    ToggleMemoryOptimization,
    Quit,
}

/// Shell state displayed in the Dioxus UI.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ShellState {
    pub app_version: String,
    pub current_profile: String,
    pub memory_profile: String,
    pub update_status: String,
    pub unread_count: u32,
    pub teams_status: TeamsStatus,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum TeamsStatus {
    #[default]
    Loading,
    Ready,
    Error(String),
}

impl std::fmt::Display for TeamsStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeamsStatus::Loading => write!(f, "Loading..."),
            TeamsStatus::Ready => write!(f, "Connected"),
            TeamsStatus::Error(e) => write!(f, "Error: {}", e),
        }
    }
}
