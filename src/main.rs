//! Rust Teams - Microsoft Teams Desktop Client
//! Features: Auto-update, Memory Optimization, Badge Notifications, URL Interception

mod app;
mod bot;
mod config;
mod error;
mod memory;
mod ui;
mod updater;

use std::error::Error;
use std::sync::{Arc, Mutex, Mutex as SyncMutex, OnceLock};


use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::{WindowBuilder, Icon};
#[cfg(target_os = "windows")]
use tao::platform::windows::WindowExtWindows;
use wry::{WebViewBuilder, WebViewBuilderExtWindows, NewWindowResponse, NewWindowFeatures};

use app::AppConfig;
use config::ConfigManager;
use ui::auto_read::get_auto_read_script;
use ui::badge::{parse_unread_count, play_notification_sound, update_taskbar_badge};
use ui::browser::open_url_smart;
use ui::browser::open_in_new_window;
use ui::console::auto_hide_console;
use ui::performance::get_all_optimization_scripts;
use ui::command_bar::get_command_bar_script;
use bot::{CommandRegistry, parse_command};

/// Global cached CommandRegistry — created once on first IPC call, reused for all subsequent commands.
static BOT_REGISTRY: OnceLock<CommandRegistry> = OnceLock::new();

fn bot_registry() -> &'static CommandRegistry {
    BOT_REGISTRY.get_or_init(CommandRegistry::new)
}

struct WebViewPtr(*const wry::WebView);
unsafe impl Send for WebViewPtr {}
unsafe impl Sync for WebViewPtr {}

static WEBVIEW: SyncMutex<Option<WebViewPtr>> = SyncMutex::new(None);

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Print version
    println!("🦀 R Teams v{}", updater::current_version());

    // Parse CLI args (--memory-profile safe|balanced|aggressive|off)
    let cli_args: Vec<String> = std::env::args().collect();
    let cli_profile = memory::parse_cli_profile(&cli_args);
    if let Some(p) = cli_profile {
        eprintln!("💾 CLI memory profile override: {}", p.as_str());
    }

    // Check for updates synchronously (before hiding console)
    println!("🔄 Checking for updates...");
    match updater::check_for_update() {
        Ok(Some(update)) => {
            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║  UPDATE AVAILABLE: v{} → v{}", updater::current_version(), update.version);
            println!("╚══════════════════════════════════════════════════════════════╝");
            println!();
            println!("   Download URL: {}", update.download_url);
            println!();
            println!("   Auto-downloading update...");
            
            if let Err(e) = updater::download_and_install(&update) {
                println!("❌ Update failed: {}", e);
                println!("   Please download manually from:");
                println!("   {}", update.download_url);
            }
        }
        Ok(None) => {
            println!("✅ Already on latest version (v{})", updater::current_version());
        }
        Err(e) => {
            println!("⚠️  Could not check for updates: {}", e);
        }
    }
    println!();

    // Load or create config
    let config_manager = ConfigManager::new();
    let mut config = match config_manager.load() {
        Ok(cfg) => {
            eprintln!("✅ Config loaded");
            cfg
        }
        Err(e) => {
            eprintln!("⚠️  Config error: {}. Using defaults.", e);
            config_manager.default_config()
        }
    };

    // Apply CLI memory profile override (nếu có)
    if let Some(profile) = cli_profile {
        profile.apply_to(&mut config.memory_optimization);
    }

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

    // Get window handle for badge updates
    #[cfg(target_os = "windows")]
    let hwnd = window.hwnd() as isize;
    #[cfg(not(target_os = "windows"))]
    let hwnd = 0isize;

    // Shared state for badge count
    let badge_count = Arc::new(Mutex::new(0u32));
    let badge_count_clone = badge_count.clone();
    let hwnd_clone = hwnd;

    // Config manager wrapped in Arc for the IPC handler
    let _config_manager: Arc<ConfigManager> = Arc::new(config_manager);

    // Build WebView with memory optimization and title change handler
    let auto_read_js = get_auto_read_script();
    let perf_js = get_all_optimization_scripts();
    let command_bar_js = get_command_bar_script();

    // Build Chromium / WebView2 browser flags từ memory config
    let browser_args = memory::build_browser_args(&config.memory_optimization);
    memory::log_summary(&config.memory_optimization);
    if !browser_args.is_empty() {
        log::info!("WebView2 args: {}", browser_args);
    }

    let mut webview_builder = WebViewBuilder::new()
        .with_url(&teams_url)
        .with_additional_browser_args(&browser_args)
        .with_initialization_script(&auto_read_js)
        .with_initialization_script(&perf_js)
        .with_initialization_script(&command_bar_js)
        .with_document_title_changed_handler(move |title: String| {
            if let Some(count) = parse_unread_count(&title) {
                let mut current_count = badge_count_clone.lock().unwrap();
                if *current_count != count {
                    *current_count = count;
                    log::info!("Title changed: '{}' → {} unread", title, count);

                    // Update taskbar badge
                    update_taskbar_badge(hwnd_clone, count);

                    // Play sound for new messages
                    if count > 0 {
                        play_notification_sound();
                    }
                }
            }
        })
        .with_new_window_req_handler(|url: String, _features: NewWindowFeatures| {
            log::info!("Intercepted navigation: {}", url);

            let lower = url.to_lowercase();

            // ---- Teams pop-out detection (chat / profile / channel) ----
            // WebView2 is single-window, so any Teams internal popup
            // (1:1 chat, group chat, profile, channel) must be opened
            // in a separate Edge window. Otherwise `NewWindowResponse::Allow`
            // would just defer to default browser (often no-op for R Teams).
            let is_teams_internal = lower.contains("teams.microsoft.com")
                || lower.contains("teams.live.com");
            let is_popout = lower.contains("/l/chat/")
                || lower.contains("/l/person/")
                || lower.contains("/l/channel/")
                || lower.contains("users=");

            if is_teams_internal && is_popout {
                log::info!("Routing Teams pop-out to new Edge window: {}", url);
                if let Err(e) = open_in_new_window(&url) {
                    log::warn!("Failed to open in new window, fallback: {}", e);
                    let _ = open_url_smart(&url);
                }
                return NewWindowResponse::Deny;
            }

            // ---- Meet/call join URLs ----
            // Open in system browser because WebView2 is single-window
            // and cannot create popup windows for the call stage.
            if lower.contains("/meet/")
                || lower.contains("/call/")
                || lower.contains("meetup-join")
                || lower.contains("teams.live.com/meet")
            {
                log::info!("Routing meet/call URL to system browser: {}", url);
                if let Err(e) = open_url_smart(&url) {
                    log::warn!("Failed to open meet URL: {}", e);
                }
                return NewWindowResponse::Deny;
            }

            // ---- Teams/Microsoft URLs (non-popout) ----
            if is_teams_internal {
                // Allow Teams URLs - open in same window
                NewWindowResponse::Allow
            } else {
                // ---- External URL ----
                if let Err(e) = open_url_smart(&url) {
                    log::warn!("Failed to open URL: {}", e);
                }
                NewWindowResponse::Deny
            }
        })
         .with_ipc_handler(move |message| {
            let body = message.body();
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&body) {
                let msg_type = msg["type"].as_str().unwrap_or("");

                if msg_type == "bot_command" {
                    let command_str = msg["data"]["command"].as_str().unwrap_or("");
                    log::info!("Bot command: {}", command_str);
                    let registry = bot_registry();
                    let (cmd, args) = match parse_command(command_str) {
                        Some((c, a)) => (c, a),
                        None => ("", ""),
                    };
                    let result = if cmd.is_empty() {
                        let list: Vec<String> = registry.commands().iter()
                            .map(|c| format!("/{} — {}", c.name, c.description))
                            .collect();
                        bot::commands::CommandResult { output: list.join("\n") }
                    } else {
                        registry.execute(cmd, args)
                    };
                    if let Ok(guard) = WEBVIEW.lock() {
                        if let Some(WebViewPtr(ptr)) = *guard {
                            let wv = unsafe { &*ptr };
                            // For autoread command, also trigger the JS function
                            if cmd == "autoread" {
                                let _ = wv.evaluate_script("processChats()");
                            }
                            let js = format!(
                                "window.dispatchEvent(new CustomEvent('rteams-bot-response', {{ detail: {{ output: '{}' }} }}));",
                                result.output.replace('\\', "\\\\").replace('\'', "\\'"),
                            );
                            let _ = wv.evaluate_script(&js);
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

    let webview_handle = Arc::new(webview);
    *WEBVIEW.lock().unwrap() = Some(WebViewPtr(&*webview_handle as *const wry::WebView));

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
    eprintln!();
    eprintln!("💡 Console will hide in 10 seconds...");

    // Auto-hide console after 10 seconds
    auto_hide_console(10000);

    let _webview_keepalive = webview_handle;

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
