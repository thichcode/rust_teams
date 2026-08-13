//! One reusable secondary window for a Teams chat.

use std::error::Error;

use tao::dpi::{LogicalPosition, LogicalSize};
use tao::event_loop::EventLoopWindowTarget;
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

/// A secondary Teams window that can be navigated between chats.
pub struct ChatWindow {
    webview: wry::WebView,
    window: tao::window::Window,
}

impl ChatWindow {
    /// Build a secondary window with an already configured WebView builder.
    pub fn create<'a>(
        event_loop: &EventLoopWindowTarget<super::AppEvent>,
        builder: WebViewBuilder<'a>,
    ) -> Result<Self, Box<dyn Error>> {
        let window = WindowBuilder::new()
            .with_title("R Teams Chat")
            .with_inner_size(LogicalSize::new(900.0, 700.0))
            .with_visible(false)
            .with_focused(false)
            .build(event_loop)
            .map_err(|error| -> Box<dyn Error> {
                format!("Failed to create chat window: {error}").into()
            })?;
        let webview = builder.build(&window).map_err(|error| -> Box<dyn Error> {
            format!("Failed to create chat WebView: {error}").into()
        })?;

        Ok(Self { webview, window })
    }

    /// Navigate the retained window to another chat and bring it to the front.
    pub fn navigate_and_focus(&self, url: &str) -> wry::Result<()> {
        self.webview.load_url(url)?;
        self.window.set_minimized(false);
        self.window.set_visible(true);
        self.window.set_focus();
        Ok(())
    }

    pub fn window_id(&self) -> tao::window::WindowId {
        self.window.id()
    }

    /// Set the window position (logical coordinates, before DPI scaling).
    pub fn set_position(&self, x: f64, y: f64) {
        self.window.set_outer_position(LogicalPosition::new(x, y));
    }
}
