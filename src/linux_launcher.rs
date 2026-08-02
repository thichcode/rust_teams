//! Linux Chromium app-mode launcher.
//!
//! WebKitGTK on Ubuntu 22.04 (2.40) is too old for the modern Teams SPA.
//! As a fallback we launch Teams in a Chromium-based browser app-mode
//! window (`--app=<url>`), which gives a clean app-like window with full
//! Chromium rendering support.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Base directory holding the downloaded portable Chromium.
pub fn chromium_home() -> Result<PathBuf, String> {
    let base = directories::ProjectDirs::from("", "", "rust-teams")
        .map(|d| d.cache_dir().to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    Ok(base.join("chromium"))
}

/// Chrome-for-Testing manifest (latest stable) with direct linux64 download URL.
const CHROMIUM_MANIFEST_URL: &str = "https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions-with-downloads.json";

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

/// Resolve a usable Chromium binary: prefer an installed browser, otherwise
/// download a portable Chrome (Chrome for Testing) if it is not cached yet.
pub fn ensure_chromium() -> Result<String, String> {
    if let Some(browser) = find_chromium_browser() {
        return Ok(browser);
    }
    let exe = chromium_home()?.join("chrome-linux64").join("chrome");
    if exe.is_file() {
        // Already downloaded — still fix permissions (upgrades from older
        // versions may have been extracted without exec bits).
        #[cfg(unix)]
        {
            let _ = make_tree_executable(&chromium_home()?.join("chrome-linux64"));
        }
        if let Ok(meta) = std::fs::metadata(&exe) {
            if meta.len() > 10_000_000 {
                return Ok(exe.display().to_string());
            }
        }
    }
    download_chromium()?;
    Ok(exe.display().to_string())
}

/// Download and extract a portable Chrome (Chrome for Testing) into the cache dir.
fn download_chromium() -> Result<(), String> {
    let home = chromium_home()?;
    fs::create_dir_all(&home).map_err(|e| format!("Cannot create {}: {e}", home.display()))?;

    // 1. Resolve the linux64 download URL from the manifest.
    let manifest: serde_json::Value = reqwest::blocking::get(CHROMIUM_MANIFEST_URL)
        .map_err(|e| format!("Failed to fetch Chrome manifest: {e}"))?
        .json()
        .map_err(|e| format!("Failed to parse Chrome manifest: {e}"))?;
    let url = manifest
        .pointer("/channels/Stable/downloads/chrome")
        .and_then(|arr| arr.as_array())
        .and_then(|arr| {
            arr.iter().find(|e| {
                matches!(
                    e.get("platform").and_then(|p| p.as_str()),
                    Some("linux64") | Some("linux-x64")
                )
            })
        })
        .and_then(|e| e.get("url"))
        .and_then(|u| u.as_str())
        .ok_or_else(|| "No linux64 Chrome download available in manifest".to_string())?;

    let version = manifest
        .pointer("/channels/Stable/version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    log::info!("Downloading portable Chrome {version} for Teams...");

    // 2. Download the zip into the cache dir.
    let zip_path = home.join("chrome-linux64.zip");
    let mut resp = reqwest::blocking::get(url).map_err(|e| format!("Download failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Download failed with HTTP {}", resp.status()));
    }
    let mut out = fs::File::create(&zip_path)
        .map_err(|e| format!("Cannot write {}: {e}", zip_path.display()))?;
    std::io::copy(&mut resp, &mut out).map_err(|e| format!("Download interrupted: {e}"))?;

    // 3. Extract.
    extract_zip(&zip_path, &home)?;

    // 4. Mark everything executable.
    // Chrome ships multiple helper binaries (chrome_crashpad_handler, and the
    // *.so subprocesses) that must be executable. The zip crate does not
    // restore the Unix exec bit, so apply +x recursively to the whole tree.
    let chrome_dir = home.join("chrome-linux64");
    if !chrome_dir.join("chrome").is_file() {
        return Err("Portable Chrome archive did not contain chrome-linux64/chrome".to_string());
    }
    #[cfg(unix)]
    if let Err(e) = make_tree_executable(&chrome_dir) {
        return Err(format!(
            "Failed to set executable bits on {}: {e}",
            chrome_dir.display()
        ));
    }

    let _ = fs::remove_file(&zip_path);
    println!(
        "✅ Portable Chrome ready at {}",
        chrome_dir.join("chrome").display()
    );
    Ok(())
}

/// Recursively add the owner +x bit to every file/dir under `root`.
#[cfg(unix)]
fn make_tree_executable(root: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let meta = entry.metadata()?;
        let mut perms = meta.permissions();
        if perms.mode() & 0o111 == 0 {
            perms.set_mode(perms.mode() | 0o111);
            fs::set_permissions(&path, perms)?;
        }
        if path.is_dir() {
            make_tree_executable(&path)?;
        }
    }
    Ok(())
}

/// Extract every entry of a zip archive into `dest`.
fn extract_zip(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let out_path = dest.join(&name);
        if entry.is_dir() {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("Cannot create {}: {e}", out_path.display()))?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Cannot create {}: {e}", parent.display()))?;
            }
            let mut f = fs::File::create(&out_path)
                .map_err(|e| format!("Cannot write {}: {e}", out_path.display()))?;
            std::io::copy(&mut entry, &mut f).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// True if the current process is running as UID 0 (root).
/// Chrome refuses to run as root without `--no-sandbox`.
pub fn running_as_root() -> bool {
    std::process::Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
}

/// Get a snap-compatible profile path for Chromium.
///
/// Snap-packaged Chromium cannot write to arbitrary directories inside `.config`
/// due to sandbox restrictions. This function detects snap Chromium and returns
/// a compatible path.
pub fn snap_compatible_profile_path(browser: &str) -> PathBuf {
    let is_snap =
        browser.starts_with("/snap/") || browser == "chromium" || browser == "chromium-browser";

    if is_snap {
        // Snap Chromium can write to its own data directory
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        home.join("snap")
            .join("chromium")
            .join("common")
            .join("rust-teams-profile")
    } else {
        // For apt/flatpak Chromium, use the standard location
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        home.join(".rust-teams").join("chromium-profile")
    }
}

/// Launch the app in Chromium mode with an isolated profile.
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
