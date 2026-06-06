//! Browser module - open links without blocking the WebView2 thread

use std::process::Command;

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

/// Smart URL opening — non-blocking, no browser detection.
/// Teams/Microsoft URLs are handled in-app; everything else opens in default browser.
pub fn open_url_smart(url: &str) -> Result<(), String> {
    if url.contains("teams.microsoft.com") || url.contains("microsoft.com") {
        return Ok(()); // Let WebView handle it
    }
    open_in_default_browser(url)
}

/// Open URL in a new Edge window (non-blocking).
/// Tries well-known Edge paths first, falls back to default browser.
/// Used for Teams chat / profile / channel pop-outs.
pub fn open_in_new_window(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // Try well-known Edge install paths (fast file existence check + non-blocking spawn)
        let edge_paths = [
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        ];

        for edge_path in &edge_paths {
            if std::path::Path::new(edge_path).exists() {
                log::info!("Opening URL in new Edge window: {}", url);
                Command::new(edge_path)
                    .arg("--new-window")
                    .arg(url)
                    .spawn()
                    .map_err(|e| format!("Failed to spawn msedge: {}", e))?;
                return Ok(());
            }
        }

        // Edge not found — fall back to default browser (non-blocking)
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
