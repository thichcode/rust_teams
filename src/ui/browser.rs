//! Browser module — open links in default or user-chosen browser

use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "windows")]
use std::ffi::OsStr;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
#[cfg(target_os = "windows")]
use std::ptr;

use reqwest::Url;
#[cfg(target_os = "windows")]
use winapi::um::shellapi::ShellExecuteW;
#[cfg(target_os = "windows")]
use winapi::um::winuser::SW_SHOWNORMAL;

/// Preferred browser path (None = system default).
/// Shared between navigation handler and IPC handler.
pub static BROWSER_PATH: OnceLock<Mutex<Option<String>>> = OnceLock::new();

/// Open URL in the system default browser (non-blocking)
pub fn open_in_default_browser(url: &str) -> Result<(), String> {
    log::info!("Opening in default browser: {}", url);

    #[cfg(target_os = "windows")]
    {
        let operation: Vec<u16> = OsStr::new("open").encode_wide().chain(Some(0)).collect();
        let target: Vec<u16> = OsStr::new(url).encode_wide().chain(Some(0)).collect();
        let result = unsafe {
            ShellExecuteW(
                ptr::null_mut(),
                operation.as_ptr(),
                target.as_ptr(),
                ptr::null(),
                ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        if result as isize <= 32 {
            return Err(format!(
                "ShellExecuteW failed with code {}",
                result as isize
            ));
        }
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
/// Teams URLs are handled in-app; everything else opens in the
/// configured browser (if provided) or the system default.
pub fn open_url_smart(url: &str, browser_path: Option<&str>) -> Result<(), String> {
    if !is_browser_web_url(url) {
        return Err("Only HTTP(S) URLs can be opened in a browser".to_owned());
    }
    if is_teams_web_url(url) {
        return Ok(()); // Let WebView handle it
    }
    match browser_path {
        Some(path) => open_with_browser(url, path),
        None => open_in_default_browser(url),
    }
}

fn is_browser_web_url(raw_url: &str) -> bool {
    let Ok(url) = Url::parse(raw_url) else {
        return false;
    };
    matches!(url.scheme(), "http" | "https") && url.host_str().is_some()
}

fn is_teams_web_url(raw_url: &str) -> bool {
    let Ok(url) = Url::parse(raw_url) else {
        return false;
    };
    if url.scheme() != "https" {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };

    host == "teams.microsoft.com"
        || host.ends_with(".teams.microsoft.com")
        || host == "teams.live.com"
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

#[cfg(test)]
mod tests {
    use super::{is_browser_web_url, is_teams_web_url};

    #[test]
    fn only_teams_hosts_stay_inside_the_webview() {
        assert!(is_teams_web_url("https://teams.microsoft.com/v2/"));
        assert!(is_teams_web_url("https://teams.live.com/v2/"));
        assert!(!is_teams_web_url("http://teams.microsoft.com/v2/"));
        assert!(!is_teams_web_url(
            "https://enterpriseenrollment.manage.microsoft.com/"
        ));
        assert!(!is_teams_web_url(
            "https://teams.microsoft.com.evil.example/v2/"
        ));
    }

    #[test]
    fn windows_browser_launcher_does_not_invoke_a_command_shell() {
        let source = include_str!("browser.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        assert!(!production.contains("Command::new(\"cmd\")"));
    }

    #[test]
    fn browser_launcher_accepts_only_http_urls() {
        assert!(is_browser_web_url("https://example.com/path?a=1&b=2"));
        assert!(is_browser_web_url("http://example.com/"));
        assert!(!is_browser_web_url("file:///C:/Windows/System32/calc.exe"));
        assert!(!is_browser_web_url("javascript:alert(1)"));
        assert!(!is_browser_web_url("not a url"));
    }
}
