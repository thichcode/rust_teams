//! Browser module — open links in default or user-chosen browser

use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

/// Preferred browser path (None = system default).
/// Shared between navigation handler and IPC handler.
pub static BROWSER_PATH: OnceLock<Mutex<Option<String>>> = OnceLock::new();

/// Open URL in the system default browser (non-blocking)
pub fn open_in_default_browser(url: &str) -> Result<(), String> {
    log::info!("Opening in default browser: {}", url);

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    Ok(())
}

/// Open URL in the given browser executable (non-blocking).
/// Falls back to default browser if the path is missing or spawn fails.
pub fn open_with_browser(url: &str, browser_path: &str) -> Result<(), String> {
    let path = Path::new(browser_path);
    if !path.exists() {
        log::warn!("Browser not found at {}, using default", browser_path);
        return open_in_default_browser(url);
    }
    log::info!("Opening URL in {}: {}", browser_path, url);
    Command::new(path)
        .arg(url)
        .spawn()
        .map(|_| ())
        .or_else(|e| {
            log::warn!(
                "Failed to spawn {} ({}), using default browser",
                browser_path,
                e
            );
            open_in_default_browser(url)
        })
}

/// Smart URL opening — non-blocking.
/// Teams/Microsoft URLs are handled in-app; everything else opens in the
/// configured browser (if provided) or the system default.
pub fn open_url_smart(url: &str, browser_path: Option<&str>) -> Result<(), String> {
    if url.contains("teams.microsoft.com") || url.contains("microsoft.com") {
        return Ok(()); // Let WebView handle it
    }
    match browser_path {
        Some(path) => open_with_browser(url, path),
        None => open_in_default_browser(url),
    }
}

/// Open URL in a new Edge window (non-blocking).
/// Tries well-known Edge paths first, falls back to default browser.
/// Used for Teams chat / profile / channel pop-outs.
pub fn open_in_new_window(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let edge_paths = [
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        ];

        for edge_path in &edge_paths {
            if Path::new(edge_path).exists() {
                log::info!("Opening URL in new Edge window: {}", url);
                Command::new(edge_path)
                    .arg("--new-window")
                    .arg(url)
                    .spawn()
                    .map_err(|e| format!("Failed to spawn msedge: {}", e))?;
                return Ok(());
            }
        }

        log::warn!("Edge not found at standard paths, using default browser");
        open_in_default_browser(url)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to xdg-open: {}", e))?;
        Ok(())
    }
}
