//! Rust Teams - Microsoft Teams Desktop Client
//! Features: Auto-update, Memory Optimization, Badge Notifications, URL Interception

mod app;
mod bot;
mod config;
mod memory;
mod ui;
mod updater;

use std::error::Error;
use std::sync::{Arc, Mutex, OnceLock};

use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
#[cfg(target_os = "windows")]
use tao::platform::windows::WindowExtWindows;
use tao::window::{Icon, WindowBuilder};
use wry::{NewWindowFeatures, NewWindowResponse, WebViewBuilder, WebViewBuilderExtWindows};

use app::AppConfig;
use bot::{CommandRegistry, parse_command};
use config::ConfigManager;
use ui::auto_read::get_auto_read_script;
use ui::badge::{parse_unread_count, play_notification_sound, update_taskbar_badge};
use ui::browser::{BROWSER_PATH, handle_browser_command, open_in_new_window, open_url_smart};
use ui::command_bar::get_command_bar_script;
use ui::console::auto_hide_console;
use ui::performance::get_all_optimization_scripts;

/// Global cached CommandRegistry — created once on first IPC call, reused for all subsequent commands.
static BOT_REGISTRY: OnceLock<CommandRegistry> = OnceLock::new();

fn bot_registry() -> &'static CommandRegistry {
    BOT_REGISTRY.get_or_init(CommandRegistry::new)
}

/// WebView stored once after build, safe for the event loop lifetime.
/// wry::WebView is !Send+!Sync (contains RefCell<HWND>), but is accessed
/// exclusively from the event-loop thread where the IPC callback runs.
struct WebViewHandle(Arc<wry::WebView>);
unsafe impl Send for WebViewHandle {}
unsafe impl Sync for WebViewHandle {}

static WEBVIEW: OnceLock<WebViewHandle> = OnceLock::new();

/// Cached update check result — avoids calling GitHub API twice.
static UPDATE_RESULT: OnceLock<updater::UpdateCheck> = OnceLock::new();

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

    // Check for updates synchronously (once, cached for later use)
    println!("🔄 Checking for updates...");
    let update_check = match updater::check_for_update() {
        Ok(Some(update)) => {
            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!(
                "║  UPDATE AVAILABLE: v{} → v{}",
                updater::current_version(),
                update.version
            );
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
            updater::UpdateCheck::Available(update)
        }
        Ok(None) => {
            println!(
                "✅ Already on latest version (v{})",
                updater::current_version()
            );
            updater::UpdateCheck::Latest
        }
        Err(e) => {
            println!("⚠️  Could not check for updates: {}", e);
            updater::UpdateCheck::Error(e)
        }
    };
    let _ = UPDATE_RESULT.set(update_check);
    println!();

    // Load or create config
    let config_manager = Arc::new(ConfigManager::new());
    let mut config = match config_manager.load() {
        Ok(cfg) => {
            let _ = BROWSER_PATH.get_or_init(|| Mutex::new(cfg.browser_path.clone()));
            eprintln!("✅ Config loaded");
            cfg
        }
        Err(e) => {
            let _ = BROWSER_PATH.get_or_init(|| Mutex::new(None));
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
        .with_title(format!("R Teams v{}", updater::current_version()))
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

    let cm_for_ipc = config_manager.clone();
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
            let browser = BROWSER_PATH.get()
                .and_then(|m| m.lock().ok())
                .and_then(|g| g.clone());

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
                    let _ = open_url_smart(&url, browser.as_deref());
                }
                return NewWindowResponse::Deny;
            }

            if lower.contains("/meet/")
                || lower.contains("/call/")
                || lower.contains("meetup-join")
                || lower.contains("teams.live.com/meet")
            {
                log::info!("Routing meet/call URL to system browser: {}", url);
                if let Err(e) = open_url_smart(&url, browser.as_deref()) {
                    log::warn!("Failed to open meet URL: {}", e);
                }
                return NewWindowResponse::Deny;
            }

            if is_teams_internal {
                NewWindowResponse::Allow
            } else {
                if let Err(e) = open_url_smart(&url, browser.as_deref()) {
                    log::warn!("Failed to open URL: {}", e);
                }
                NewWindowResponse::Deny
            }
        })
         .with_ipc_handler(move |message| {
            let body = message.body();
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(body) {
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
                    if let Some(wv) = WEBVIEW.get() {
                        if cmd == "autoread" {
                            let _ = wv.0.evaluate_script("processChats()");
                        }
                        if cmd == "browser" {
                            let output = handle_browser_command(args);
                            if let (Some(path), Some(bp)) = (output.new_browser, BROWSER_PATH.get())
                            {
                                *bp.lock().unwrap() = path;
                                if let Ok(mut cfg) = cm_for_ipc.load() {
                                    cfg.browser_path = BROWSER_PATH.get()
                                        .and_then(|m| m.lock().ok())
                                        .and_then(|g| g.clone());
                                    let _ = cm_for_ipc.save(&cfg);
                                }
                            }
                            let js = format!(
                                "window.dispatchEvent(new CustomEvent('rteams-bot-response', {{ detail: {{ output: '{}' }} }}));",
                                output.message.replace('\\', "\\\\").replace('\'', "\\'"),
                            );
                            let _ = wv.0.evaluate_script(&js);
                            return;
                        }
                        let js = format!(
                            "window.dispatchEvent(new CustomEvent('rteams-bot-response', {{ detail: {{ output: '{}' }} }}));",
                            result.output.replace('\\', "\\\\").replace('\'', "\\'"),
                        );
                        let _ = wv.0.evaluate_script(&js);
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

    #[allow(clippy::arc_with_non_send_sync)]
    let wv_arc = Arc::new(webview);
    let _ = WEBVIEW.set(WebViewHandle(wv_arc.clone()));
    let _webview_keepalive = wv_arc;

    // Use cached update check result (no second API call)
    let version_info = UPDATE_RESULT
        .get()
        .map(|r| r.version_info())
        .unwrap_or_else(|| format!("📦 Version: v{}", updater::current_version()));

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

    // Capture config + manager for save-on-close
    let cm_for_save = config_manager;
    let mut config_for_save = config.clone();

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
                #[cfg(target_os = "windows")]
                unsafe {
                    use winapi::um::winuser::GetWindowRect;
                    let mut rect = std::mem::zeroed();
                    if GetWindowRect(hwnd as _, &mut rect) != 0 {
                        config_for_save.window_settings.width = (rect.right - rect.left) as u32;
                        config_for_save.window_settings.height = (rect.bottom - rect.top) as u32;
                    }
                }
                if let Err(e) = cm_for_save.save(&config_for_save) {
                    log::warn!("Failed to save config on close: {e}");
                }
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
            let is_r = (8..=24).contains(&x)
                && (6..=26).contains(&y)
                && ((x <= 12) || (y <= 10) || (y >= 18 && x >= 12 && (x + y) <= 32));

            if is_r {
                // White for "R"
                rgba[idx] = 255;
                rgba[idx + 1] = 255;
                rgba[idx + 2] = 255;
                rgba[idx + 3] = 255;
            } else {
                // Teams purple background
                rgba[idx] = 98; // R
                rgba[idx + 1] = 100; // G
                rgba[idx + 2] = 167; // B
                rgba[idx + 3] = 255; // A
            }
        }
    }

    let icon =
        Icon::from_rgba(rgba, size, size).map_err(|e| format!("Failed to create icon: {}", e))?;

    Ok(icon)
}

/// Get the Teams URL from config profiles, or return the default Teams URL
fn get_teams_url(config: &AppConfig) -> String {
    if let Some(ref profile_id) = config.current_profile_id
        && let Some(profile) = config.profiles.iter().find(|p| &p.id == profile_id)
    {
        return profile.teams_url.clone();
    }

    if let Some(profile) = config.profiles.iter().find(|p| p.is_default) {
        return profile.teams_url.clone();
    }

    if let Some(profile) = config.profiles.first() {
        return profile.teams_url.clone();
    }

    "https://teams.microsoft.com".to_string()
}
