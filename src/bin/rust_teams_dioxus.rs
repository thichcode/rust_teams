//! R Teams Dioxus Shell - Proof of Concept
//! A Dioxus-based UI shell for Microsoft Teams with native look and feel.

use dioxus::prelude::*;

use rust_teams::app::AppConfig;
use rust_teams::config::ConfigManager;
use rust_teams::shell::bridge::{ShellCommand, ShellState, TeamsStatus};
use rust_teams::shell::layout::ShellApp;

fn main() {
    env_logger::init();
    println!("🦀 R Teams Dioxus Shell v{}", rust_teams::updater::current_version());
    dioxus::launch(app);
}

fn app() -> Element {
    let config_manager = ConfigManager::new();
    let config = match config_manager.load() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("⚠️  Config error: {}. Using defaults.", e);
            config_manager.default_config()
        }
    };

    let teams_url = get_teams_url(&config);
    let memory_label = if config.memory_optimization.enabled { "ON" } else { "OFF" };
    let profile_name = config
        .profiles
        .iter()
        .find(|p| p.is_default)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "Default".to_string());

    let shell_state = ShellState {
        app_version: rust_teams::updater::current_version().to_string(),
        current_profile: profile_name,
        memory_profile: memory_label.to_string(),
        update_status: "Checking...".to_string(),
        teams_status: TeamsStatus::Loading,
        ..Default::default()
    };

    let shell_state = use_signal(|| shell_state);
    let teams_url_signal = use_signal(|| teams_url);

    let _teams_handle = use_coroutine(move |_rx: UnboundedReceiver<ShellCommand>| {
        let url = teams_url_signal.read().clone();
        async move {
            spawn_teams_webview(&url).await;
        }
    });

    rsx! {
        ShellApp {
            state: shell_state.read().clone(),
            on_command: move |cmd| {
                match cmd {
                    ShellCommand::ReloadTeams => {
                        let url = teams_url_signal.read().clone();
                        spawn(async move {
                            spawn_teams_webview(&url).await;
                        });
                    }
                    ShellCommand::Quit => {
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn spawn_teams_webview(url: &str) {
    log::info!("Teams WebView requested for: {}", url);
    let url = url.to_string();
    std::thread::spawn(move || {
        if let Err(e) = run_teams_window(&url) {
            log::error!("Teams window error: {}", e);
        }
    });
}

fn run_teams_window(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    use dioxus_desktop::tao::event::{Event, StartCause, WindowEvent};
    use dioxus_desktop::tao::event_loop::{ControlFlow, EventLoopBuilder};
    use dioxus_desktop::tao::window::WindowBuilder;
    use dioxus_desktop::wry::WebViewBuilder;

    let event_loop = EventLoopBuilder::<()>::with_user_event().build();
    let window = WindowBuilder::new()
        .with_title("R Teams - Microsoft Teams")
        .with_inner_size(dioxus_desktop::tao::dpi::LogicalSize::new(1200.0, 800.0))
        .build(&event_loop)?;

    let webview = WebViewBuilder::new()
        .with_url(url)
        .with_devtools(true)
        .with_initialization_script(INIT_SCRIPT)
        .with_navigation_handler(|url| {
            let lower = url.to_lowercase();
            if lower.contains("teams.microsoft.com") || lower.contains("teams.live.com") {
                true
            } else if lower.starts_with("http://") || lower.starts_with("https://") {
                let _ = webbrowser::open(&url);
                false
            } else {
                true
            }
        })
        .with_ipc_handler(|message| {
            let body = message.body();
            log::info!("Teams IPC: {}", body);
        })
        .build(&window)?;

    let _webview = std::rc::Rc::new(webview);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => {
                log::info!("Teams window initialized");
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

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

const INIT_SCRIPT: &str = r#"
(function () {
    'use strict';
    const style = document.createElement('style');
    style.textContent = `
        * { animation-duration: 0s !important; transition-duration: 0s !important; }
    `;
    document.head.appendChild(style);

    if (window.chrome && window.chrome.webview) {
        window.chrome.webview.postMessage(JSON.stringify({
            type: 'page_loaded',
            data: { url: location.href }
        }));
    }
})();
"#;
