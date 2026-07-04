#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod config;
mod diagnostics;
mod export;
mod diarize;
mod download;
mod hotkey;
mod notes;
mod stt;
mod suggest;
mod translate;
mod tray;
mod vad;

use app::MeetingAssistantApp;
use config::Config;

fn main() -> eframe::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let config = Config::load();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([500.0, 400.0])
            .with_title("R Teams Meeting Assistant"),
        ..Default::default()
    };

    let tray = tray::TrayManager::new();
    let tray_rx = tray.rx;

    let hotkey = hotkey::HotkeyManager::new(&config.toggle_hotkey);
    let hotkey_rx = hotkey.rx;

    eframe::run_native(
        "R Teams Meeting Assistant",
        native_options,
        Box::new(|cc| {
            let _ = &cc.egui_ctx;
            Ok(Box::new(MeetingAssistantApp::new(cc, config, Some(tray_rx), Some(hotkey_rx))))
        }),
    )
}
