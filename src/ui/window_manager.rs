//! Multi-window manager — handles creating and managing multiple WebView windows

use tao::event_loop::EventLoop;
use tao::window::WindowBuilder;
use wry::{WebView, WebViewBuilder, WebViewBuilderExtWindows};

use crate::app::MemoryOptimization;

#[allow(dead_code)]
/// A managed window with its WebView
pub struct ManagedWindow {
    pub title: String,
    pub url: String,
    pub webview: WebView,
}

#[allow(dead_code)]
/// Manages multiple application windows
pub struct WindowManager {
    pub windows: Vec<ManagedWindow>,
    memory_config: MemoryOptimization,
}

#[allow(dead_code)]
impl WindowManager {
    pub fn new(memory_config: MemoryOptimization) -> Self {
        Self {
            windows: Vec::new(),
            memory_config,
        }
    }

    /// Create a new window with a WebView loading the given URL
    pub fn create_window(
        &mut self,
        event_loop: &EventLoop<()>,
        title: &str,
        url: &str,
    ) -> Result<(), String> {
        let window = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(tao::dpi::LogicalSize::new(1200.0, 800.0))
            .build(event_loop)
            .map_err(|e| format!("Failed to create window: {}", e))?;

        let mut builder = WebViewBuilder::new().with_url(url);

        // Apply memory optimization settings
        if self.memory_config.enabled {
            builder = builder
                .with_default_context_menus(false)
                .with_devtools(true);
        }

        let webview = builder
            .build(&window)
            .map_err(|e| format!("Failed to create WebView: {}", e))?;

        log::info!("Created new window: '{}' → {}", title, url);

        self.windows.push(ManagedWindow {
            title: title.to_string(),
            url: url.to_string(),
            webview,
        });

        Ok(())
    }

    /// Create a new window with the Teams URL
    pub fn create_teams_window(
        &mut self,
        event_loop: &EventLoop<()>,
        teams_url: &str,
    ) -> Result<(), String> {
        let count = self.windows.len() + 1;
        let title = if count == 1 {
            "Rust Teams".to_string()
        } else {
            format!("Rust Teams #{}", count)
        };

        self.create_window(event_loop, &title, teams_url)
    }

    /// Get the number of open windows
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// Check if we should limit new windows (memory protection)
    pub fn can_create_window(&self) -> bool {
        // Limit to 5 windows max to prevent excessive memory usage
        self.windows.len() < 5
    }
}
