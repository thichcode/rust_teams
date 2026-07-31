//! Linux Chromium app-mode launcher.
//!
//! WebKitGTK on Ubuntu 22.04 (2.40) is too old for the modern Teams SPA.
//! As a fallback we launch Teams in a Chromium-based browser app-mode
//! window (`--app=<url>`), which gives a clean app-like window with full
//! Chromium rendering support.

use std::path::PathBuf;
use std::process::Command;

/// Known Chromium-based browser executable names (checked on PATH).
const BROWSER_CANDIDATES: &[&str] = &[
    "google-chrome",
    "google-chrome-stable",
    "chromium",
    "chromium-browser",
    "microsoft-edge",
    "microsoft-edge-stable",
    "brave-browser",
    "brave",
];

/// Additional absolute paths checked as a fallback.
const BROWSER_PATHS: &[&str] = &[
    "/usr/bin/google-chrome",
    "/usr/bin/google-chrome-stable",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
    "/snap/bin/chromium",
    "/snap/bin/google-chrome",
    "/snap/bin/microsoft-edge",
    "/opt/google/chrome/chrome",
];

/// Find an installed Chromium-based browser, returning its executable name or path.
pub fn find_chromium_browser() -> Option<String> {
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            for name in BROWSER_CANDIDATES {
                if dir.join(name).is_file() {
                    return Some(name.to_string());
                }
            }
        }
    }

    BROWSER_PATHS
        .iter()
        .find(|p| std::path::Path::new(p).is_file())
        .map(|p| p.to_string())
}

/// Launch the URL in Chromium app-mode with an isolated profile.
///
/// `extra_args` are additional Chromium flags (e.g. `--disable-gpu`).
/// Returns the spawned child on success.
pub fn launch_app_mode(
    browser: &str,
    url: &str,
    extra_args: &[String],
    user_data_dir: &PathBuf,
) -> Result<(), String> {
    let mut cmd = Command::new(browser);
    cmd.arg(format!("--app={url}"))
        .arg(format!("--user-data-dir={}", user_data_dir.display()))
        .arg("--no-first-run")
        .arg("--no-default-browser-check");
    for flag in extra_args {
        cmd.arg(flag);
    }
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to launch {}: {e}", browser))
}
