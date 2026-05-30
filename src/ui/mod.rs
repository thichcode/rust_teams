//! UI module — Window management, WebView, and multi-window support

pub mod auto_read;
pub mod badge;
pub mod performance;
pub mod window_manager;

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
pub use window_manager::WindowManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSettings {
    pub width: u32,
    pub height: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub maximized: bool,
}
