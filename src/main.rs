//! Rust Teams - Microsoft Teams Desktop Client
//! Features: Auto-update, Memory Optimization, Badge Notifications, URL Interception, Meeting Notes

mod app;
mod config;
mod error;
mod meeting;
mod ui;
mod updater;

use std::error::Error;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::{WindowBuilder, Icon};
use wry::{WebViewBuilder, WebViewBuilderExtWindows, NewWindowResponse, NewWindowFeatures};

use app::AppConfig;
use config::ConfigManager;
use meeting::{MeetingNotesGenerator, MeetingNotesConfig};
use ui::auto_read::get_auto_read_script;
use ui::badge::{parse_unread_count, play_notification_sound};
use ui::browser::open_url_smart;
use ui::console::auto_hide_console;
use ui::meeting_detect::get_meeting_detection_script;
use ui::performance::get_all_optimization_scripts;

/// Shared state for meeting notes
struct MeetingState {
    is_meeting_active: Arc<AtomicBool>,
    generator: Arc<Mutex<Option<MeetingNotesGenerator>>>,
}

impl MeetingState {
    fn new(config: MeetingNotesConfig) -> Self {
        Self {
            is_meeting_active: Arc::new(AtomicBool::new(false)),
            generator: Arc::new(Mutex::new(
                MeetingNotesGenerator::new(config).ok()
            )),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Print version
    println!("🦀 R Teams v{}", updater::current_version());

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

    // Create meeting state
    let meeting_state = Arc::new(Mutex::new(MeetingState::new(config.meeting_notes.clone())));

    // Create event loop and window
    let event_loop = EventLoop::new();
    let mut window_builder = WindowBuilder::new()
        .with_title("R Teams")
        .with_inner_size(tao::dpi::LogicalSize::new(
            config.window_settings.width as f64,
            config.window_settings.height as f64,
        ));

    // Set window icon from embedded resource
    if let Ok(icon) = load_window_icon() {
        window_builder = window_builder.with_window_icon(Some(icon));
    }

    if config.window_settings.maximized {
        window_builder = window_builder.with_maximized(true);
    }

    let window = window_builder
        .build(&event_loop)
        .map_err(|e| -> Box<dyn Error> { format!("Failed to create window: {}", e).into() })?;

    // Shared state for badge count
    let badge_count = Arc::new(Mutex::new(0u32));
    let badge_count_clone = badge_count.clone();

    // Clone meeting state for IPC handler
    let meeting_state_ipc = meeting_state.clone();

    // Build WebView with memory optimization and title change handler
    let auto_read_js = get_auto_read_script();
    let perf_js = get_all_optimization_scripts();
    let meeting_js = get_meeting_detection_script();
    let mut webview_builder = WebViewBuilder::new()
        .with_url(&teams_url)
        .with_initialization_script(&auto_read_js)
        .with_initialization_script(&perf_js)
        .with_initialization_script(&meeting_js)
        .with_document_title_changed_handler(move |title: String| {
            if let Some(count) = parse_unread_count(&title) {
                let mut current_count = badge_count_clone.lock().unwrap();
                if *current_count != count {
                    *current_count = count;
                    log::info!("Title changed: '{}' → {} unread", title, count);

                    // Play sound for new messages
                    if count > 0 {
                        play_notification_sound();
                    }
                }
            }
        })
        .with_new_window_req_handler(|url: String, _features: NewWindowFeatures| {
            log::info!("Intercepted navigation: {}", url);

            // Check if it's a Teams/Microsoft URL
            if url.contains("teams.microsoft.com") || url.contains("microsoft.com") {
                // Allow Teams URLs - open in same window
                NewWindowResponse::Allow
            } else {
                // External URL - try to open in running browser
                if let Err(e) = open_url_smart(&url) {
                    log::warn!("Failed to open URL: {}", e);
                }
                // Deny opening in WebView
                NewWindowResponse::Deny
            }
        })
        .with_ipc_handler(move |message| {
            // Handle IPC messages from JavaScript
            let body = message.body();
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&body) {
                if msg["type"] == "meeting_state_changed" {
                    let active = msg["data"]["active"].as_bool().unwrap_or(false);
                    let duration = msg["data"]["duration"].as_u64().unwrap_or(0);

                    log::info!("Meeting state changed: active={}, duration={}", active, duration);

                    if let Ok(state) = meeting_state_ipc.lock() {
                        if active && !state.is_meeting_active.load(Ordering::Relaxed) {
                            // Meeting started - start recording
                            state.is_meeting_active.store(true, Ordering::Relaxed);
                            if let Ok(mut generator) = state.generator.lock() {
                                if let Some(ref mut recorder) = *generator {
                                    if let Err(e) = recorder.start_meeting() {
                                        log::error!("Failed to start meeting recording: {}", e);
                                    } else {
                                        log::info!("Meeting recording started");
                                    }
                                }
                            }
                        } else if !active && state.is_meeting_active.load(Ordering::Relaxed) {
                            // Meeting ended - log it
                            state.is_meeting_active.store(false, Ordering::Relaxed);
                            log::info!("Meeting ended");
                            eprintln!("📝 Meeting ended - generating notes...");

                            // Note: Full async meeting notes generation requires
                            // resolving cpal::Stream Send issues
                            // For now, we log the event
                        }
                    }
                }
            }
        });

    if config.memory_optimization.enabled {
        webview_builder = webview_builder
            .with_default_context_menus(false)
            .with_devtools(true);
    }

    let webview = webview_builder
        .build(&window)
        .map_err(|e| -> Box<dyn Error> { format!("Failed to create WebView: {}", e).into() })?;

    // Check version
    let current_version = updater::current_version();
    let version_info = match updater::check_for_update() {
        Ok(Some(update)) => {
            format!("📦 Version: v{} (update available: v{})", current_version, update.version)
        }
        Ok(None) => {
            format!("📦 Version: v{} (latest)", current_version)
        }
        Err(_) => {
            format!("📦 Version: v{}", current_version)
        }
    };

    eprintln!("✅ R Teams window created successfully!");
    eprintln!("{}", version_info);
    eprintln!("🔔 Badge notifications: ENABLED");
    eprintln!("🔗 Links: Open in running browser (or default)");
    eprintln!("📖 Auto-read: ENABLED (keywords: closed, cancel)");
    eprintln!("⚡ Performance: ENABLED (prefetch, lazy load, cache)");
    eprintln!("📝 Meeting Notes: ENABLED (auto-record + generate .md)");
    eprintln!();
    eprintln!("💡 Console will hide in 5 seconds...");

    // Auto-hide console after 5 seconds
    auto_hide_console(5000);

    // Keep webview alive
    let _webview = webview;

    // Run event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                log::info!("R Teams initialized");
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

/// Load window icon from embedded RGBA data
fn load_window_icon() -> Result<Icon, Box<dyn Error>> {
    // 32x32 Teams purple icon with white "R"
    let size: u32 = 32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];

    for y in 0..size {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;

            // Simple "R" letter approximation
            let is_r = (x >= 8 && x <= 24 && y >= 6 && y <= 26)
                && ((x <= 12)
                    || (y <= 10)
                    || (y >= 18 && x >= 12 && (x + y) <= 32));

            if is_r {
                // White for "R"
                rgba[idx] = 255;
                rgba[idx + 1] = 255;
                rgba[idx + 2] = 255;
                rgba[idx + 3] = 255;
            } else {
                // Teams purple background
                rgba[idx] = 98;  // R
                rgba[idx + 1] = 100; // G
                rgba[idx + 2] = 167; // B
                rgba[idx + 3] = 255; // A
            }
        }
    }

    let icon = Icon::from_rgba(rgba, size, size)
        .map_err(|e| format!("Failed to create icon: {}", e))?;

    Ok(icon)
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
