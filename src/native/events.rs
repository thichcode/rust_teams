use serde::{Deserialize, Serialize};

/// Events from Teams WebView to shell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TeamsEvent {
    PageLoaded,
    TitleChanged(String),
    NavigationStarted(String),
    NavigationCompleted(String),
    Error(String),
}

/// Events from shell to Teams WebView.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShellToTeams {
    Reload,
    Navigate(String),
}
