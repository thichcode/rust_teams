//! Rust Teams - Microsoft Teams Desktop Client
//! Features: Auto-update, Memory Optimization, Badge Notifications, URL Interception

mod app;
mod config;
#[cfg(not(target_os = "windows"))]
mod linux_launcher;
mod memory;
mod ui;
mod updater;

use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex, OnceLock};

use reqwest::Url;
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy};
#[cfg(target_os = "windows")]
use tao::platform::windows::WindowExtWindows;
use tao::window::{Icon, WindowBuilder};
#[cfg(target_os = "windows")]
use wry::WebViewBuilderExtWindows;
#[cfg(target_os = "windows")]
use wry::WebViewExtWindows;
use wry::{NewWindowFeatures, NewWindowResponse, WebViewBuilder};

use app::{AppConfig, LinuxBackend, WebkitRenderMode};
use config::ConfigManager;

const CHROME_UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";
use ui::AppEvent;
use ui::auto_read::get_auto_read_script;
use ui::badge::{parse_unread_count, play_notification_sound, update_taskbar_badge};
use ui::browser::{BROWSER_PATH, open_in_new_window, open_url_smart};
use ui::chat_popout::{
    get_chat_popout_script, is_teams_chat_url, is_teams_meeting_url, is_trusted_teams_url,
};
use ui::chat_window::ChatWindow;
use ui::console::auto_hide_console;
use ui::performance::{get_all_optimization_scripts, get_visibility_script};

/// Cached update check result — avoids calling GitHub API twice.
static UPDATE_RESULT: OnceLock<updater::UpdateCheck> = OnceLock::new();

fn parse_webview_event(body: &str) -> Option<AppEvent> {
    let message = serde_json::from_str::<serde_json::Value>(body).ok()?;
    let message_type = message.get("type")?.as_str()?;
    let url = message.get("data")?.get("url")?.as_str()?;

    match message_type {
        "open_chat" if is_teams_chat_url(url) => Some(AppEvent::OpenChat(url.to_owned())),
        "open_external" => {
            let parsed = Url::parse(url).ok()?;
            if matches!(parsed.scheme(), "http" | "https") && !is_trusted_teams_url(url) {
                Some(AppEvent::OpenExternal(url.to_owned()))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn queue_webview_event(body: &str, proxy: &EventLoopProxy<AppEvent>) {
    let Some(event) = parse_webview_event(body) else {
        log::warn!("Ignored invalid WebView IPC message");
        return;
    };
    if let Err(error) = proxy.send_event(event) {
        log::error!("Failed to queue WebView event: {error}");
    }
}

fn handle_navigation(url: String) -> bool {
    log::info!("Navigation: {url}");

    if is_trusted_teams_url(&url) {
        return true;
    }

    if let Ok(parsed) = Url::parse(&url) {
        let host = parsed.host_str().unwrap_or("");
        if host == "login.microsoftonline.com"
            || host.ends_with(".microsoftonline.com")
            || host == "login.live.com"
            || host.ends_with(".login.live.com")
            || host == "account.live.com"
            || host.ends_with(".account.live.com")
            || host == "www.microsoft.com"
            || host == "support.microsoft.com"
        {
            return true;
        }
    }

    if url.starts_with("http://") || url.starts_with("https://") {
        let browser = BROWSER_PATH
            .get()
            .and_then(|value| value.lock().ok())
            .and_then(|value| value.clone());

        log::info!("Opening external URL in browser: {url}");
        if let Err(error) = open_url_smart(&url, browser.as_deref()) {
            log::warn!("Failed to open external URL: {error}");
        }
        return false;
    }

    true
}

fn looks_like_chat_request(raw_url: &str) -> bool {
    let Ok(url) = Url::parse(raw_url) else {
        return false;
    };

    let path = url.path().to_ascii_lowercase();
    path.starts_with("/l/chat/")
        || path.starts_with("/chat/")
        || (matches!(path.as_str(), "/v2" | "/v2/")
            && url
                .query_pairs()
                .any(|(key, value)| key == "users" || (key == "ctx" && value == "chat")))
}

fn handle_new_window_request(url: String, proxy: &EventLoopProxy<AppEvent>) -> NewWindowResponse {
    log::info!("Intercepted navigation: {url}");

    if is_teams_chat_url(&url) {
        log::info!("Opening Teams chat in the secondary window: {url}");
        if let Err(error) = proxy.send_event(AppEvent::OpenChat(url)) {
            log::error!("Failed to queue secondary chat window: {error}");
        }
        return NewWindowResponse::Deny;
    }

    let lower = url.to_lowercase();
    let browser = BROWSER_PATH
        .get()
        .and_then(|value| value.lock().ok())
        .and_then(|value| value.clone());
    let is_teams_internal = is_trusted_teams_url(&url);
    let is_popout = lower.contains("/l/person/") || lower.contains("/l/channel/");

    if is_teams_internal && is_popout {
        log::info!("Routing Teams pop-out to a new Edge window: {url}");
        if let Err(error) = open_in_new_window(&url) {
            log::warn!("Failed to open in a new window: {error}");
            let _ = open_url_smart(&url, browser.as_deref());
        }
        return NewWindowResponse::Deny;
    }

    if is_teams_meeting_url(&url) {
        log::info!("Opening Teams meeting in secondary window: {url}");
        if let Err(error) = proxy.send_event(AppEvent::OpenMeeting(url)) {
            log::error!("Failed to queue meeting window: {error}");
        }
        return NewWindowResponse::Deny;
    }

    if looks_like_chat_request(&url) {
        log::info!("Routing chat-shaped popup to secondary window (loose match): {url}");
        if let Err(error) = proxy.send_event(AppEvent::OpenChat(url)) {
            log::error!("Failed to queue secondary chat window: {error}");
        }
        return NewWindowResponse::Deny;
    }

    if is_teams_internal {
        NewWindowResponse::Allow
    } else {
        if let Err(error) = open_url_smart(&url, browser.as_deref()) {
            log::warn!("Failed to open URL: {error}");
        }
        NewWindowResponse::Deny
    }
}

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

    // Handle --install-chromium (exit early, do not open the app)
    if handle_install_chromium(&cli_args) {
        return Ok(());
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

    // Parse --render-mode from CLI
    let cli_render_mode = parse_render_mode(&cli_args);
    if let Some(mode) = cli_render_mode {
        eprintln!("🎨 CLI render mode override: {:?}", mode);
        config.webkit_render_mode = mode;
    }

    // Apply WebKitGTK environment variables (Linux only, harmless on other platforms)
    let render_mode = config.webkit_render_mode;
    match render_mode {
        WebkitRenderMode::Compat => {
            // SAFETY: called once at startup before any threads; single-threaded context
            unsafe {
                std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
                std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
                std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "true");
            }
            log::info!("WebKit render mode: COMPAT (software rendering)");
        }
        WebkitRenderMode::Auto => {
            log::info!("WebKit render mode: AUTO (hardware compositing)");
        }
    }

    // Determine Teams URL (override with --url for diagnostics)
    let teams_url = parse_cli_url(&cli_args).unwrap_or_else(|| get_teams_url(&config));
    eprintln!("🌐 Teams URL: {}", teams_url);

    // Parse --backend from CLI (auto | webkit | chromium)
    if let Some(backend) = parse_cli_backend(&cli_args) {
        eprintln!("🚀 CLI backend override: {:?}", backend);
        config.linux_backend = backend;
    }

    // On non-Windows: decide backend; Chromium app-mode may exit early.
    if try_launch_chromium_backend(&config, &teams_url)? {
        return Ok(());
    }

    eprintln!(
        "🧠 Memory optimization: {}",
        if config.memory_optimization.enabled {
            "ON"
        } else {
            "OFF"
        }
    );

    // Create event loop and window
    let event_loop: EventLoop<AppEvent> = EventLoopBuilder::<AppEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
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

    let main_window_id = window.id();

    // Shared state for badge count
    let badge_count = Arc::new(Mutex::new(0u32));
    let badge_count_clone = badge_count.clone();
    let hwnd_clone = hwnd;

    // Build WebView with memory optimization and title change handler
    let vis_js = get_visibility_script();
    let auto_read_js = get_auto_read_script();
    let perf_js = get_all_optimization_scripts();
    let chat_popout_js = get_chat_popout_script();

    // Build Chromium / WebView2 browser flags từ memory config
    let browser_args = memory::build_browser_args(&config.memory_optimization);
    memory::log_summary(&config.memory_optimization);
    if !browser_args.is_empty() {
        log::info!("WebView2 args: {}", browser_args);
    }

    let proxy_for_popouts = proxy.clone();
    let proxy_for_ipc = proxy.clone();
    let mut webview_builder = WebViewBuilder::new()
        .with_url(&teams_url)
        .with_user_agent(CHROME_UA);

    #[cfg(target_os = "windows")]
    {
        webview_builder = webview_builder.with_additional_browser_args(&browser_args);
    }

    webview_builder = webview_builder
        .with_initialization_script(&vis_js)
        .with_initialization_script(&auto_read_js)
        .with_initialization_script(&perf_js)
        .with_initialization_script(&chat_popout_js)
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
        .with_navigation_handler(handle_navigation)
        .with_new_window_req_handler(move |url: String, _features: NewWindowFeatures| {
            handle_new_window_request(url, &proxy_for_popouts)
        })
        .with_ipc_handler(move |message| {
            queue_webview_event(message.body(), &proxy_for_ipc);
        });

    if config.memory_optimization.enabled {
        #[cfg(target_os = "windows")]
        {
            webview_builder = webview_builder.with_default_context_menus(false);
        }
        webview_builder = webview_builder.with_devtools(true);
    }

    let webview = webview_builder
        .build(&window)
        .map_err(|e| -> Box<dyn Error> { format!("Failed to create WebView: {}", e).into() })?;

    #[cfg(target_os = "windows")]
    let webview_environment = webview.environment();

    #[allow(clippy::arc_with_non_send_sync)]
    let wv_arc = Arc::new(webview);
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
    #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
    let mut config_for_save = config.clone();

    let mut chat_windows: HashMap<String, ChatWindow> = HashMap::new();
    let mut meeting_windows: HashMap<String, ChatWindow> = HashMap::new();

    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                log::info!("R Teams initialized");
            }
            Event::UserEvent(AppEvent::OpenExternal(url)) => {
                let browser = BROWSER_PATH
                    .get()
                    .and_then(|value| value.lock().ok())
                    .and_then(|value| value.clone());
                log::info!("Opening external link from WebView IPC: {url}");
                if let Err(error) = open_url_smart(&url, browser.as_deref()) {
                    log::warn!("Failed to open external URL: {error}");
                }
            }
            Event::UserEvent(AppEvent::OpenChat(url)) => {
                let needs_new = match chat_windows.get(&url) {
                    Some(window) => window.navigate_and_focus(&url).is_err(),
                    None => true,
                };

                if needs_new {
                    chat_windows.remove(&url);
                    let proxy_for_secondary = proxy.clone();
                    let proxy_for_secondary_ipc = proxy.clone();
                    let builder = WebViewBuilder::new()
                        .with_user_agent(CHROME_UA)
                        .with_initialization_script(chat_popout_js.clone())
                        .with_navigation_handler(handle_navigation)
                        .with_new_window_req_handler(
                            move |url: String, _features: NewWindowFeatures| {
                                handle_new_window_request(url, &proxy_for_secondary)
                            },
                        )
                        .with_ipc_handler(move |message| {
                            queue_webview_event(message.body(), &proxy_for_secondary_ipc);
                        });
                    #[cfg(target_os = "windows")]
                    let builder = builder.with_environment(webview_environment.clone());

                    match ChatWindow::create(event_loop, builder) {
                        Ok(window) => {
                            let offset = (chat_windows.len() as f64) * 35.0;
                            window.set_position(100.0 + offset, 100.0 + offset);
                            match window.navigate_and_focus(&url) {
                                Ok(()) => {
                                    log::info!("Opened chat window #{}", chat_windows.len() + 1);
                                    chat_windows.insert(url, window);
                                }
                                Err(error) => {
                                    log::error!(
                                        "Failed to navigate the new secondary chat window: {error}"
                                    );
                                }
                            }
                        }
                        Err(error) => {
                            log::error!("Failed to create secondary chat window: {error}");
                        }
                    }
                }
            }
            Event::UserEvent(AppEvent::OpenMeeting(url)) => {
                let needs_new = match meeting_windows.get(&url) {
                    Some(window) => window.navigate_and_focus(&url).is_err(),
                    None => true,
                };

                if needs_new {
                    meeting_windows.remove(&url);
                    let proxy_for_secondary = proxy.clone();
                    let proxy_for_secondary_ipc = proxy.clone();
                    let builder = WebViewBuilder::new()
                        .with_user_agent(CHROME_UA)
                        .with_initialization_script(chat_popout_js.clone())
                        .with_navigation_handler(handle_navigation)
                        .with_new_window_req_handler(
                            move |url: String, _features: NewWindowFeatures| {
                                handle_new_window_request(url, &proxy_for_secondary)
                            },
                        )
                        .with_ipc_handler(move |message| {
                            queue_webview_event(message.body(), &proxy_for_secondary_ipc);
                        });
                    #[cfg(target_os = "windows")]
                    let builder = builder.with_environment(webview_environment.clone());

                    match ChatWindow::create(event_loop, builder) {
                        Ok(window) => {
                            let offset = (meeting_windows.len() as f64) * 35.0;
                            window.set_position(200.0 + offset, 200.0 + offset);
                            match window.navigate_and_focus(&url) {
                                Ok(()) => {
                                    log::info!(
                                        "Opened meeting window #{}",
                                        meeting_windows.len() + 1
                                    );
                                    meeting_windows.insert(url, window);
                                }
                                Err(error) => {
                                    log::error!(
                                        "Failed to navigate the new meeting window: {error}"
                                    );
                                }
                            }
                        }
                        Err(error) => {
                            log::error!("Failed to create meeting window: {error}");
                        }
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
                ..
            } => {
                if window_id == main_window_id {
                    log::info!("Main window close requested, shutting down...");
                    #[cfg(target_os = "windows")]
                    unsafe {
                        use winapi::um::winuser::GetWindowRect;
                        let mut rect = std::mem::zeroed();
                        if GetWindowRect(hwnd as _, &mut rect) != 0 {
                            config_for_save.window_settings.width = (rect.right - rect.left) as u32;
                            config_for_save.window_settings.height =
                                (rect.bottom - rect.top) as u32;
                        }
                    }
                    if let Err(e) = cm_for_save.save(&config_for_save) {
                        log::warn!("Failed to save config on close: {e}");
                    }
                    *control_flow = ControlFlow::Exit;
                } else {
                    let prev = chat_windows.len();
                    chat_windows.retain(|_, w| w.window_id() != window_id);
                    if chat_windows.len() < prev {
                        log::info!(
                            "Secondary chat window closed ({} remaining)",
                            chat_windows.len()
                        );
                    }
                    meeting_windows.retain(|_, w| w.window_id() != window_id);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::Destroyed,
                window_id,
                ..
            } if window_id == main_window_id => {
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

/// Parse `--url <url>` from CLI args (diagnostic override).
fn parse_cli_url(args: &[String]) -> Option<String> {
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "--url" {
            return iter.next().cloned();
        }
    }
    None
}

/// Parse `--backend auto|webkit|chromium` from CLI args.
fn parse_cli_backend(args: &[String]) -> Option<LinuxBackend> {
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "--backend" {
            return match iter.next()?.as_str() {
                "auto" => Some(LinuxBackend::Auto),
                "webkit" => Some(LinuxBackend::Webkit),
                "chromium" => Some(LinuxBackend::Chromium),
                _ => {
                    eprintln!("⚠️  Unknown backend. Use: auto | webkit | chromium");
                    None
                }
            };
        }
    }
    None
}

/// Try to launch Teams in a Chromium app-mode window.
///
/// Returns `Ok(true)` if the Chromium backend was used (app exits),
/// `Ok(false)` if we should continue with the embedded WebView.
#[cfg(target_os = "windows")]
fn try_launch_chromium_backend(_config: &AppConfig, _url: &str) -> Result<bool, Box<dyn Error>> {
    Ok(false)
}

/// Try to launch Teams in a Chromium app-mode window (non-Windows).
///
/// On `Auto`/`Chromium` the app prefers an installed Chromium-based browser,
/// else downloads a portable Chrome (Chrome for Testing) automatically. If that
/// fails we fall back to the embedded WebKitGTK webview.
#[cfg(not(target_os = "windows"))]
fn try_launch_chromium_backend(config: &AppConfig, url: &str) -> Result<bool, Box<dyn Error>> {
    let want_chromium = match config.linux_backend {
        LinuxBackend::Webkit => false,
        LinuxBackend::Chromium | LinuxBackend::Auto => true,
    };

    if !want_chromium {
        log::info!("Backend: embedded WebKitGTK webview");
        return Ok(false);
    }

    let browser = match linux_launcher::ensure_chromium() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("⚠️  No usable Chromium browser ({e}). Falling back to WebKitGTK.");
            eprintln!(
                "   Tip: install google-chrome, chromium, or microsoft-edge (or use --backend webkit to silence)."
            );
            log::warn!("ensure_chromium failed, using WebKitGTK: {e}");
            return Ok(false);
        }
    };

    let mut flags: Vec<String> = memory::build_browser_args(&config.memory_optimization)
        .split_whitespace()
        .map(str::to_string)
        .collect();

    // Chrome refuses to run as root without --no-sandbox (common in containers/VDI).
    if linux_launcher::running_as_root() {
        flags.push("--no-sandbox".to_string());
        log::info!("Running as root — adding --no-sandbox");
    }

    let user_data_dir = linux_launcher::snap_compatible_profile_path(&browser);
    eprintln!(
        "🚀 Launching Teams in {} app-mode (profile: {}) ...",
        browser,
        user_data_dir.display()
    );
    log::info!("Launching Chromium backend: {} --app={}", browser, url);

    match linux_launcher::launch_app_mode(&browser, url, &flags, &user_data_dir) {
        Ok(()) => {
            log::info!("Chromium app-mode launched successfully");
            Ok(true)
        }
        Err(e) => {
            eprintln!("⚠️  Chromium launch failed ({e}). Falling back to WebKitGTK.");
            log::warn!("Chromium launch failed, falling back to WebKitGTK: {e}");
            Ok(false)
        }
    }
}

/// Handle `--install-chromium`: on Windows does nothing (no-op).
#[cfg(target_os = "windows")]
fn handle_install_chromium(_args: &[String]) -> bool {
    false
}

/// Handle `--install-chromium`: installs the Chromium package via apt (needs sudo).
///
/// Returns `true` when the flag was present (caller should exit after this).
#[cfg(not(target_os = "windows"))]
fn handle_install_chromium(args: &[String]) -> bool {
    if !args.iter().any(|a| a == "--install-chromium") {
        return false;
    }

    println!("🌐 Installing Chromium via apt (requires sudo)...");
    match std::process::Command::new("sudo")
        .args(["apt-get", "install", "-y", "chromium-browser"])
        .status()
    {
        Ok(status) if status.success() => {
            println!("✅ Chromium installed. Verify: chromium-browser --version");
        }
        Ok(status) => {
            eprintln!(
                "❌ apt install failed (exit {:?}). Run manually:\n   sudo apt-get install -y chromium-browser",
                status.code()
            );
        }
        Err(e) => {
            eprintln!(
                "❌ Could not run sudo: {e}\n   Run manually: sudo apt-get install -y chromium-browser"
            );
        }
    }
    true
}

/// Parse `--render-mode auto|compat` from CLI args.
fn parse_render_mode(args: &[String]) -> Option<WebkitRenderMode> {
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "--render-mode" {
            match iter.next()?.as_str() {
                "compat" => return Some(WebkitRenderMode::Compat),
                "auto" => return Some(WebkitRenderMode::Auto),
                _ => {
                    eprintln!("⚠️  Unknown render mode. Use: auto | compat");
                    return None;
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod popup_routing_tests {
    use super::{handle_navigation, looks_like_chat_request, parse_webview_event};
    use crate::ui::AppEvent;

    #[test]
    fn allows_login_live_com_navigation() {
        assert!(handle_navigation("https://login.live.com/".into()));
    }

    #[test]
    fn allows_login_microsoftonline_com_navigation() {
        assert!(handle_navigation(
            "https://login.microsoftonline.com/".into()
        ));
    }

    #[test]
    fn blocks_external_url_navigation() {
        assert!(!handle_navigation("https://example.com/".into()));
    }

    #[test]
    fn parses_chat_open_ipc_event() {
        let body = r#"{"type":"open_chat","data":{"url":"https://teams.microsoft.com/v2/?ctx=chat&chatId=19%3Aabc%40thread.v2"}}"#;

        match parse_webview_event(body) {
            Some(AppEvent::OpenChat(url)) => assert!(url.contains("chatId=19%3Aabc")),
            event => panic!("unexpected event: {event:?}"),
        }
    }

    #[test]
    fn parses_external_microsoft_link_ipc_event() {
        let body = r#"{"type":"open_external","data":{"url":"https://enterpriseenrollment.manage.microsoft.com/"}}"#;

        match parse_webview_event(body) {
            Some(AppEvent::OpenExternal(url)) => {
                assert_eq!(url, "https://enterpriseenrollment.manage.microsoft.com/")
            }
            event => panic!("unexpected event: {event:?}"),
        }
    }

    #[test]
    fn rejects_non_web_external_ipc_event() {
        let body = r#"{"type":"open_external","data":{"url":"javascript:alert(1)"}}"#;

        assert!(parse_webview_event(body).is_none());
    }

    #[test]
    fn rejects_query_hint_on_external_admin_path() {
        assert!(!looks_like_chat_request(
            "https://example.com/admin?users=alice"
        ));
    }

    #[test]
    fn recognizes_http_teams_v2_query_hint_without_trusting_it() {
        assert!(looks_like_chat_request(
            "http://teams.microsoft.com/v2?users=alice"
        ));
    }

    #[test]
    fn recognizes_direct_untrusted_chat_path() {
        assert!(looks_like_chat_request("https://evil.example/l/chat/0/0"));
    }

    #[test]
    fn rejects_normal_external_url_as_chat_shape() {
        assert!(!looks_like_chat_request("https://example.com/docs"));
    }

    #[test]
    fn rejects_query_hint_on_person_path() {
        assert!(!looks_like_chat_request(
            "https://teams.microsoft.com/l/person/alice?users=bob@example.com"
        ));
    }
}
