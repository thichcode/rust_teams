//! Rust Teams - Microsoft Teams Desktop Client
//! Features: Auto-update, Memory Optimization

mod app;
mod config;
mod error;
mod ui;
mod updater;

use std::error::Error;

use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::WindowBuilder;
use wry::{WebViewBuilder, WebViewBuilderExtWindows};

use app::AppConfig;
use config::ConfigManager;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Print version
    println!("🦀 Rust Teams v{}", updater::current_version());

    // Check for updates in background (non-blocking)
    std::thread::spawn(|| {
        updater::print_update_status();
    });

    // Load or create config
    let config_manager = ConfigManager::new();
    let config = match config_manager.load() {
        Ok(cfg) => {
            eprintln!("✅ Config loaded");
            cfg
        }
        Err(e) => {
            eprintln!("⚠️  Config error: {}. Using defaults.", e);
            config_manager.default_config()
        }
    };

    // Determine Teams URL
    let teams_url = get_teams_url(&config);
    eprintln!("🌐 Teams URL: {}", teams_url);
    eprintln!(
        "🧠 Memory optimization: {}",
        if config.memory_optimization.enabled {
            "ON"
        } else {
            "OFF"
        }
    );

    // Create event loop and window
    let event_loop = EventLoop::new();
    let mut window_builder = WindowBuilder::new()
        .with_title("Rust Teams")
        .with_inner_size(tao::dpi::LogicalSize::new(
            config.window_settings.width as f64,
            config.window_settings.height as f64,
        ));

    if config.window_settings.maximized {
        window_builder = window_builder.with_maximized(true);
    }

    let window = window_builder
        .build(&event_loop)
        .map_err(|e| -> Box<dyn Error> { format!("Failed to create window: {}", e).into() })?;

    // Build WebView with memory optimization
    let mut webview_builder = WebViewBuilder::new().with_url(&teams_url);

    if config.memory_optimization.enabled {
        webview_builder = webview_builder
            .with_default_context_menus(false)
            .with_devtools(true);
    }

    let webview = webview_builder
        .build(&window)
        .map_err(|e| -> Box<dyn Error> { format!("Failed to create WebView: {}", e).into() })?;

    eprintln!("✅ Window + WebView created successfully!");
    eprintln!("💡 Multi-window support: WindowManager ready (Ctrl+N coming soon)");

    // Keep webview alive
    let _webview = webview;

    // Run event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                log::info!("Application initialized");
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                log::info!("Close requested, shutting down...");
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Destroyed,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

/// Get the Teams URL from config profiles, or return the default Teams URL
fn get_teams_url(config: &AppConfig) -> String {
    if let Some(ref profile_id) = config.current_profile_id {
        if let Some(profile) = config.profiles.iter().find(|p| &p.id == profile_id) {
            return profile.teams_url.clone();
        }
    }

    if let Some(profile) = config.profiles.iter().find(|p| p.is_default) {
        return profile.teams_url.clone();
    }

    if let Some(profile) = config.profiles.first() {
        return profile.teams_url.clone();
    }

    "https://teams.microsoft.com".to_string()
}
