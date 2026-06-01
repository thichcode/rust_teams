//! Rust Teams - Microsoft Teams Desktop Client
//! Features: Auto-update, Memory Optimization, Badge Notifications, URL Interception, Meeting Notes, Realtime Translate

mod app;
mod config;
mod error;
mod meeting;
mod memory;
mod ui;
mod updater;

use std::error::Error;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::{WindowBuilder, Icon};
#[cfg(target_os = "windows")]
use tao::platform::windows::WindowExtWindows;
use wry::{WebView, WebViewBuilder, WebViewBuilderExtWindows, NewWindowResponse, NewWindowFeatures};

use app::AppConfig;
use config::ConfigManager;
use meeting::{MeetingNotesGenerator, MeetingNotesConfig, RealtimePayload, RealtimeTranslatePipeline};
use ui::auto_read::get_auto_read_script;
use ui::badge::{parse_unread_count, play_notification_sound, update_taskbar_badge};
use ui::browser::open_url_smart;
use ui::console::auto_hide_console;
use ui::meeting_detect::get_meeting_detection_script;
use ui::performance::get_all_optimization_scripts;
use ui::realtime_panel::get_realtime_panel_script;

/// Shared state for meeting notes
struct MeetingState {
    is_meeting_active: Arc<AtomicBool>,
    generator: Arc<Mutex<Option<MeetingNotesGenerator>>>,
    /// Pipeline drives STT -> translate -> suggestions while a call is active
    realtime_pipeline: Arc<Mutex<Option<RealtimeTranslatePipeline>>>,
    /// Receiver for the pipeline's payload stream; drained from the event loop
    realtime_rx: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<RealtimePayload>>>>,
    /// Reference to the WebView (wrapped in Arc because WebView is not Clone)
    /// so we can inject translated captions as JS
    webview: Arc<Mutex<Option<Arc<WebView>>>>,
}

impl MeetingState {
    fn new(config: MeetingNotesConfig) -> Self {
        Self {
            is_meeting_active: Arc::new(AtomicBool::new(false)),
            generator: Arc::new(Mutex::new(
                MeetingNotesGenerator::new(config).ok()
            )),
            realtime_pipeline: Arc::new(Mutex::new(None)),
            realtime_rx: Arc::new(Mutex::new(None)),
            webview: Arc::new(Mutex::new(None)),
        }
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

    // Get window handle for badge updates
    #[cfg(target_os = "windows")]
    let hwnd = window.hwnd() as isize;
    #[cfg(not(target_os = "windows"))]
    let hwnd = 0isize;

    // Shared state for badge count
    let badge_count = Arc::new(Mutex::new(0u32));
    let badge_count_clone = badge_count.clone();
    let hwnd_clone = hwnd;

    // Clone meeting state for IPC handler
    let meeting_state_ipc = meeting_state.clone();
    // Realtime translate config is needed by the IPC handler when a call starts
    let realtime_cfg_ipc: Arc<Mutex<Option<meeting::RealtimeTranslateConfig>>> =
        Arc::new(Mutex::new(Some(config.realtime_translate.clone())));

    // Auto-download whisper.cpp + model if local STT is configured
    #[cfg(target_os = "windows")]
    {
        let stt_type = config.realtime_translate.stt.provider_type.as_str();
        if stt_type == "local" || stt_type == "whisper-cpp" || stt_type == "whisper.cpp" {
            let data_dir = directories::ProjectDirs::from("com", "rust-teams", "app")
                .map(|p| p.data_dir().to_path_buf())
                .unwrap_or_else(|| std::env::temp_dir().join("rust-teams"));
            let dl = meeting::whisper_download::WhisperDownloader::new(data_dir);

            if dl.needs_download() {
                eprintln!("📥 Downloading whisper.cpp + model (~100MB)…");
                match dl.ensure_downloaded() {
                    Ok(()) => eprintln!("✅ Whisper download complete"),
                    Err(e) => eprintln!("⚠️  Whisper download failed: {}", e),
                }
            } else {
                eprintln!("✅ Whisper files already present");
            }

            // Update realtime_cfg_ipc with downloaded paths
            if dl.bin_path().exists() && dl.model_path().exists() {
                if let Ok(mut locked) = realtime_cfg_ipc.lock() {
                    if let Some(ref mut rt) = *locked {
                        if rt.stt.api_url.is_empty() {
                            rt.stt.api_url = dl.bin_path().to_string_lossy().to_string();
                        }
                        if rt.stt.api_key.is_empty() {
                            rt.stt.api_key = dl.model_path().to_string_lossy().to_string();
                        }
                    }
                }
            }
        }
    }

    // Build WebView with memory optimization and title change handler
    let auto_read_js = get_auto_read_script();
    let perf_js = get_all_optimization_scripts();
    let meeting_js = get_meeting_detection_script();
    let realtime_panel_js = get_realtime_panel_script();

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
        .with_initialization_script(&meeting_js)
        .with_initialization_script(&realtime_panel_js)
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

            // Meet/call join URLs — open in system browser because WebView2
            // is single-window and cannot create popup windows for the call stage
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

                            // Start realtime translate pipeline (only if enabled)
                            if let Ok(rt_cfg_lock) = realtime_cfg_ipc.lock() {
                                if let Some(cfg) = rt_cfg_lock.as_ref() {
                                    if cfg.enabled {
                                        let (tx, rx) =
                                            tokio::sync::mpsc::unbounded_channel::<RealtimePayload>();
                                        let pipeline = RealtimeTranslatePipeline::new(
                                            cfg.clone(),
                                            tx,
                                        );
                                        if let Err(e) = pipeline.start() {
                                            log::error!("Failed to start realtime translate: {}", e);
                                        } else {
                                            if let Ok(mut slot) = state.realtime_pipeline.lock() {
                                                *slot = Some(pipeline);
                                            }
                                            if let Ok(mut slot) = state.realtime_rx.lock() {
                                                *slot = Some(rx);
                                            }
                                            log::info!(
                                                "Realtime translate started: {} -> {}",
                                                cfg.source_lang, cfg.target_lang
                                            );
                                        }
                                    }
                                }
                            }
                        } else if !active && state.is_meeting_active.load(Ordering::Relaxed) {
                            // Meeting ended - log it
                            state.is_meeting_active.store(false, Ordering::Relaxed);
                            log::info!("Meeting ended");
                            eprintln!("📝 Meeting ended - generating notes...");

                            // Stop realtime translate pipeline
                            if let Ok(mut slot) = state.realtime_pipeline.lock() {
                                if let Some(p) = slot.as_ref() {
                                    p.stop();
                                }
                                *slot = None;
                            }
                            if let Ok(mut slot) = state.realtime_rx.lock() {
                                *slot = None;
                            }
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

    // Save the WebView into meeting state so the event loop can drive
    // `evaluate_script` calls (used to push realtime captions into the UI).
    // We share the WebView through an Arc<WebView>; wry's WebView is not Clone
    // but the inner state is ref-counted (COM on Windows), so wrapping in Arc
    // is safe and lets us hand a handle to multiple consumers.
    let webview_handle = Arc::new(webview);
    if let Ok(state) = meeting_state.lock() {
        if let Ok(mut slot) = state.webview.lock() {
            *slot = Some(webview_handle.clone());
        }
    }

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
    eprintln!(
        "🌐 Realtime translate: {} ({} -> {})",
        if config.realtime_translate.enabled { "ENABLED" } else { "disabled" },
        config.realtime_translate.source_lang,
        config.realtime_translate.target_lang
    );
    eprintln!();
    eprintln!("💡 Console will hide in 10 seconds...");

    // Auto-hide console after 10 seconds
    auto_hide_console(10000);

    // Keep webview alive for the lifetime of the event loop.
    // (webview_handle is already stored in meeting state; this just keeps
    // one strong reference alive on the main stack.)
    let _webview_keepalive = webview_handle;

    // Event-loop-side handle for draining realtime translate payloads
    let meeting_state_evt = meeting_state.clone();

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

        // Drain any pending realtime translate payloads and push them into
        // the WebView as a JS-side event.
        if let Ok(state) = meeting_state_evt.lock() {
            if let Ok(mut rx_slot) = state.realtime_rx.lock() {
                if let Some(rx) = rx_slot.as_mut() {
                    while let Ok(payload) = rx.try_recv() {
                        if let Ok(wv_slot) = state.webview.lock() {
                            if let Some(wv) = wv_slot.as_ref() {
                                if let Ok(json) = serde_json::to_string(&payload) {
                                    let escaped = json.replace('\\', "\\\\").replace('\'', "\\'");
                                    let js = format!(
                                        "window.dispatchEvent(new CustomEvent('rteams-realtime', {{ detail: JSON.parse('{}') }}));",
                                        escaped
                                    );
                                    if let Err(e) = wv.evaluate_script(&js) {
                                        log::warn!("Failed to inject realtime payload: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
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
