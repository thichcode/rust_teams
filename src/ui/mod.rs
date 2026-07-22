//! UI module — window settings, WebView helpers, and IPC-facing scripts

pub mod auto_read;
pub mod badge;
pub mod browser;
pub mod chat_popout;
pub mod chat_window;
pub mod console;
pub mod performance;

use serde::{Deserialize, Serialize};

/// Custom events from WebView callbacks to the main event loop.
#[derive(Debug, Clone)]
pub enum AppEvent {
    OpenChat(String),
    OpenMeeting(String),
    OpenExternal(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSettings {
    pub width: u32,
    pub height: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub maximized: bool,
}
