//! Browser module — open links in default or user-chosen browser

use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

/// Preferred browser path (None = system default).
/// Shared between navigation handler and IPC handler.
pub static BROWSER_PATH: OnceLock<Mutex<Option<String>>> = OnceLock::new();

/// Known browser install paths on Windows, in priority order.
const BROWSER_PATHS: &[(&str, &str)] = &[
    (
        "Chrome",
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    ),
    (
        "Chrome (x86)",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ),
    ("Firefox", r"C:\Program Files\Mozilla Firefox\firefox.exe"),
    (
        "Firefox (x86)",
        r"C:\Program Files (x86)\Mozilla Firefox\firefox.exe",
    ),
    (
        "Edge",
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    ),
    (
        "Edge",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ),
    (
        "Brave",
        r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe",
    ),
    ("Opera", r"C:\Program Files\Opera\launcher.exe"),
    (
        "Vivaldi",
        r"C:\Program Files\Vivaldi\Application\vivaldi.exe",
    ),
];

/// A detected browser with its display name and executable path.
#[derive(Debug, Clone)]
pub struct InstalledBrowser {
    pub name: String,
    pub path: String,
}

/// Result of a `/browser` command — display message + optional new browser path.
pub struct BrowserCommandOutput {
    pub message: String,
    /// Set to `Some(path)` when the user chose a specific browser,
    /// `Some("")` for default, `None` when just listing.
    pub new_browser: Option<Option<String>>,
}

/// Handle the `/browser` slash command.
///
/// - No args: List detected browsers + current setting.
/// - `list`: Same as no args.
/// - `default`: Reset to system default browser.
/// - `<name>`: Try to match a detected browser by name (case-insensitive).
pub fn handle_browser_command(args: &str) -> BrowserCommandOutput {
    let current = BROWSER_PATH
        .get()
        .and_then(|m| m.lock().ok())
        .and_then(|g| g.clone());

    let trimmed = args.trim().to_lowercase();

    if trimmed.is_empty() || trimmed == "list" {
        let mut msg = String::new();
        let browsers = detect_browsers();
        if browsers.is_empty() {
            msg.push_str("No additional browsers detected.\n");
        } else {
            msg.push_str("Installed browsers:\n");
            for b in &browsers {
                msg.push_str(&format!("  {} — {}\n", b.name, b.path));
            }
        }
        msg.push_str("\nCurrent: ");
        match &current {
            Some(p) => msg.push_str(p),
            None => msg.push_str("system default"),
        }
        msg.push_str("\n\nUsage:\n  /browser <name>  — set browser\n  /browser default — use system default\n  /browser list    — show this list");
        return BrowserCommandOutput {
            message: msg,
            new_browser: None,
        };
    }

    if trimmed == "default" {
        return BrowserCommandOutput {
            message: "✅ Links will open in the system default browser.".into(),
            new_browser: Some(None),
        };
    }

    // Try to match by name
    let browsers = detect_browsers();
    for b in &browsers {
        if b.name.to_lowercase() == trimmed {
            return BrowserCommandOutput {
                message: format!("✅ Links will open in {} ({}).", b.name, b.path),
                new_browser: Some(Some(b.path.clone())),
            };
        }
    }

    // Try partial match
    for b in &browsers {
        if b.name.to_lowercase().contains(&trimmed) {
            return BrowserCommandOutput {
                message: format!("✅ Links will open in {} ({}).", b.name, b.path),
                new_browser: Some(Some(b.path.clone())),
            };
        }
    }

    BrowserCommandOutput {
        message: format!(
            "Browser '{}' not found. Use /browser list to see options.",
            trimmed
        ),
        new_browser: None,
    }
}

/// Scan well-known install paths and return browsers that exist on disk.
pub fn detect_browsers() -> Vec<InstalledBrowser> {
    let mut found: Vec<InstalledBrowser> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (name, path) in BROWSER_PATHS {
        if Path::new(path).exists() && seen.insert(path) {
            found.push(InstalledBrowser {
                name: name.to_string(),
                path: path.to_string(),
            });
        }
    }
    found
}

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
