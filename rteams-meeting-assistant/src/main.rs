#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod config;
mod notes;
mod stt;
mod suggest;
mod translate;

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

    eframe::run_native(
        "R Teams Meeting Assistant",
        native_options,
        Box::new(|cc| {
            let _ = &cc.egui_ctx;
            Ok(Box::new(MeetingAssistantApp::new(cc, config)))
        }),
    )
}
