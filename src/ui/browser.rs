//! Browser module - detect running browsers and open links

use std::process::Command;

/// Supported browsers to detect
const BROWSERS: &[(&str, &str)] = &[
    ("chrome.exe", "Google Chrome"),
    ("msedge.exe", "Microsoft Edge"),
    ("firefox.exe", "Mozilla Firefox"),
    ("opera.exe", "Opera"),
    ("brave.exe", "Brave Browser"),
    ("vivaldi.exe", "Vivaldi"),
];

/// Detect if a browser is running
fn is_browser_running(process_name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("tasklist")
            .arg("/FI")
            .arg(format!("IMAGENAME eq {}", process_name))
            .output();
        
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.to_lowercase().contains(&process_name.to_lowercase());
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("pgrep")
            .arg("-f")
            .arg(process_name)
            .output();
        
        if let Ok(output) = output {
            return output.status.success();
        }
    }
    
    false
}

/// Find the first running browser
pub fn find_running_browser() -> Option<&'static str> {
    for (process, name) in BROWSERS {
        if is_browser_running(process) {
            log::info!("Found running browser: {}", name);
            return Some(process);
        }
    }
    None
}

/// Open URL in the system default browser
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

/// Open URL in a specific browser with new tab
pub fn open_in_browser(browser: &str, url: &str) -> Result<(), String> {
    log::info!("Opening in {}: {}", browser, url);
    
    #[cfg(target_os = "windows")]
    {
        // Try to open with specific browser
        let result = match browser {
            "chrome.exe" => Command::new("chrome")
                .arg("--new-window")
                .arg(url)
                .spawn(),
            "msedge.exe" => Command::new("msedge")
                .arg("--new-window")
                .arg(url)
                .spawn(),
            "firefox.exe" => Command::new("firefox")
                .arg("--new-window")
                .arg(url)
                .spawn(),
            "opera.exe" => Command::new("opera")
                .arg("--new-window")
                .arg(url)
                .spawn(),
            _ => return open_in_default_browser(url),
        };
        
        result.map_err(|e| format!("Failed to open URL: {}", e))?;
    }
    
    Ok(())
}

/// Smart URL opening - detect browser and open accordingly
pub fn open_url_smart(url: &str) -> Result<(), String> {
    // Check if it's a Teams/Microsoft URL - open in-app
    if url.contains("teams.microsoft.com") || url.contains("microsoft.com") {
        return Ok(()); // Let WebView handle it
    }

    // Try to find a running browser
    if let Some(browser) = find_running_browser() {
        open_in_browser(browser, url)
    } else {
        open_in_default_browser(url)
    }
}

/// Open URL in a new Edge window (force new window, not new tab).
/// Falls back to default browser if Edge not available.
/// Used for Teams chat / profile / channel pop-outs.
pub fn open_in_new_window(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // Try to find Edge via well-known install paths
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

        // Edge not found at standard paths — try `where msedge` lookup via cmd
        log::warn!("Edge not found at standard paths, trying `where msedge`");
        let lookup = Command::new("cmd")
            .args(&["/C", "where", "msedge"])
            .output();
        if let Ok(out) = lookup {
            if out.status.success() {
                let path = String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !path.is_empty() && std::path::Path::new(&path).exists() {
                    log::info!("Found Edge via where: {}", path);
                    Command::new(&path)
                        .arg("--new-window")
                        .arg(url)
                        .spawn()
                        .map_err(|e| format!("Failed to spawn msedge: {}", e))?;
                    return Ok(());
                }
            }
        }

        // Last resort: ShellExecute via cmd /C start (uses default browser)
        log::warn!("Edge not found, falling back to default browser");
        Command::new("cmd")
            .args(&["/C", "start", "", url])
            .spawn()
            .map_err(|e| format!("Failed to spawn start: {}", e))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Non-Windows: just use xdg-open
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to xdg-open: {}", e))?;
        Ok(())
    }
}
