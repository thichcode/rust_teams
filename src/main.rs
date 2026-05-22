//! Main application entry point
extern crate tokio;

use anyhow::Result;
use std::error::Error;

mod app;
mod config;
mod error;
mod ui;

pub use app::App;
pub use config::ConfigManager;
pub use error::AppError;

// Re-export DLLS for MSVC builds
extern "system" {
    fn LoadLibraryA(lpLibFileName: *const u8) -> *mut std::ffi::c_void;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::init();

    // Load config
    let config_manager = ConfigManager::new();

    match config_manager.load() {
        Ok(cfg) => {
            println!("Config loaded. Starting app...");
            let _app = App::new(cfg)?;
            println!("App started successfully.");
        }
        Err(e) => {
            eprintln!("Critical: Failed to load config. Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}