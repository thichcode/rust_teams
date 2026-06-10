//! Auto-update module — checks GitHub Releases and downloads updates

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Duration;

const REPO: &str = "thichcode/rust_teams";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 2000;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    pub body: String,
}

/// Check if a newer version is available on GitHub Releases
pub fn check_for_update() -> Result<Option<UpdateInfo>, String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("rust_teams-updater")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&url)
        .send()
        .map_err(|e| format!("Failed to check for updates: {}", e))?;

    // Handle rate limiting
    if response.status().as_u16() == 403 {
        if let Some(retry_after) = response.headers().get("Retry-After") {
            if let Ok(seconds) = retry_after.to_str().unwrap_or("60").parse::<u64>() {
                return Err(format!(
                    "GitHub API rate limited. Try again in {} seconds",
                    seconds
                ));
            }
        }
        return Err("GitHub API rate limited. Try again later.".to_string());
    }

    if !response.status().is_success() {
        return Err(format!("GitHub API returned status: {}", response.status()));
    }

    let release: serde_json::Value = response
        .json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

       let tag_name = release["tag_name"]
        .as_str()
        .ok_or("Missing tag_name")?
        .to_string();
    let tag_name = if let Some(pos) = tag_name.rfind("-v") {
        // Handle tags like "rteams-meeting-assistant-v0.4.4"
        tag_name[pos+2..].to_string()
    } else {
        // Handle standard tags like "v1.2.3"
        tag_name.trim_start_matches('v').to_string()
    };

    let current = semver::Version::parse(CURRENT_VERSION)
        .map_err(|e| format!("Invalid current version: {}", e))?;
    let latest =
        semver::Version::parse(&tag_name).map_err(|e| format!("Invalid latest version: {}", e))?;

    if latest <= current {
        log::info!("Already on latest version (v{})", CURRENT_VERSION);
        return Ok(None);
    }

    log::info!("Update available: v{} → v{}", CURRENT_VERSION, tag_name);

    // Find the exe asset - try multiple patterns
    let download_url = find_download_url(&release, &tag_name)?;

    let body = release["body"]
        .as_str()
        .unwrap_or("No release notes")
        .to_string();

    Ok(Some(UpdateInfo {
        version: tag_name,
        download_url,
        body,
    }))
}

/// Find the download URL from release assets
fn find_download_url(release: &serde_json::Value, tag: &str) -> Result<String, String> {
    // Try to find exe asset with various patterns
    if let Some(assets) = release["assets"].as_array() {
        // Priority 1: rust_teams-windows-x64.exe
        for asset in assets {
            if let Some(name) = asset["name"].as_str() {
                if name == "rust_teams-windows-x64.exe" {
                    return asset["browser_download_url"]
                        .as_str()
                        .map(|s| s.to_string())
                        .ok_or("Missing download URL".to_string());
                }
            }
        }

        // Priority 2: Any .exe with x64
        for asset in assets {
            if let Some(name) = asset["name"].as_str() {
                if name.ends_with(".exe") && name.contains("x64") {
                    return asset["browser_download_url"]
                        .as_str()
                        .map(|s| s.to_string())
                        .ok_or("Missing download URL".to_string());
                }
            }
        }

        // Priority 3: Any .exe
        for asset in assets {
            if let Some(name) = asset["name"].as_str() {
                if name.ends_with(".exe") {
                    return asset["browser_download_url"]
                        .as_str()
                        .map(|s| s.to_string())
                        .ok_or("Missing download URL".to_string());
                }
            }
        }
    }

    // Fallback URL
    Ok(format!(
        "https://github.com/{}/releases/download/v{}/rust_teams-windows-x64.exe",
        REPO, tag
    ))
}

/// Download update with retry logic
pub fn download_and_install(update: &UpdateInfo) -> Result<(), String> {
    println!("⏳ Downloading v{}...", update.version);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(300))
        .user_agent("rust_teams-updater")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Get current exe path
    let current_exe =
        std::env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;
    let exe_dir = current_exe.parent().ok_or("Failed to get exe directory")?;

    // Create temp and backup paths
    let temp_exe = exe_dir.join("rust_teams.exe.tmp");
    let backup_exe = exe_dir.join("rust_teams.exe.bak");

    // Clean up any leftover temp files
    if temp_exe.exists() {
        fs::remove_file(&temp_exe).ok();
    }

    // Download with retry
    let mut last_error = String::new();
    for attempt in 1..=MAX_RETRIES {
        println!("   Attempt {}/{}...", attempt, MAX_RETRIES);

        match download_file(&client, &update.download_url, &temp_exe) {
            Ok(size) => {
                println!("✅ Download complete ({} KB)", size / 1024);

                // Validate downloaded file
                if let Err(e) = validate_download(&temp_exe) {
                    last_error = format!("Validation failed: {}", e);
                    fs::remove_file(&temp_exe).ok();
                    if attempt < MAX_RETRIES {
                        println!("⚠️  Validation failed, retrying...");
                        std::thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
                        continue;
                    }
                    return Err(last_error);
                }

                // Install the update
                return install_update(&temp_exe, &current_exe, &backup_exe);
            }
            Err(e) => {
                last_error = e;
                fs::remove_file(&temp_exe).ok();
                if attempt < MAX_RETRIES {
                    println!("⚠️  Download failed, retrying in {}s...", RETRY_DELAY_MS / 1000);
                    std::thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
                }
            }
        }
    }

    Err(format!(
        "Download failed after {} attempts: {}",
        MAX_RETRIES, last_error
    ))
}

/// Download file to disk
fn download_file(
    client: &reqwest::blocking::Client,
    url: &str,
    path: &PathBuf,
) -> Result<u64, String> {
    let mut response = client
        .get(url)
        .send()
        .map_err(|e| format!("Failed to start download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let mut file = fs::File::create(path).map_err(|e| format!("Failed to create file: {}", e))?;

    let mut total_bytes = 0u64;
    let mut buffer = vec![0u8; 8192];
    let mut last_progress = 0u64;

    loop {
        let bytes_read = response
            .read(&mut buffer)
            .map_err(|e| format!("Read error: {}", e))?;
        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])
            .map_err(|e| format!("Write error: {}", e))?;
        total_bytes += bytes_read as u64;

        // Progress indicator (update every 100KB)
        if total_bytes - last_progress >= 102400 {
            print!("\r   Downloaded: {} KB", total_bytes / 1024);
            std::io::stdout().flush().ok();
            last_progress = total_bytes;
        }
    }
    println!();

    file.flush()
        .map_err(|e| format!("Flush error: {}", e))?;
    drop(file);

    Ok(total_bytes)
}

/// Validate downloaded file
fn validate_download(path: &PathBuf) -> Result<(), String> {
    let metadata =
        fs::metadata(path).map_err(|e| format!("Failed to read file metadata: {}", e))?;

    // Check file size (should be at least 1MB for a valid exe)
    if metadata.len() < 1024 * 1024 {
        return Err(format!(
            "File too small: {} bytes (expected at least 1MB)",
            metadata.len()
        ));
    }

    // Check PE header (Windows executable)
    let mut file =
        fs::File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut header = [0u8; 2];
    file.read_exact(&mut header)
        .map_err(|e| format!("Failed to read header: {}", e))?;

    // Check for MZ header (DOS executable)
    if header[0] != b'M' || header[1] != b'Z' {
        return Err("Not a valid Windows executable (missing MZ header)".to_string());
    }

    Ok(())
}

/// Install the update by replacing the current exe
fn install_update(
    temp_exe: &PathBuf,
    current_exe: &PathBuf,
    backup_exe: &PathBuf,
) -> Result<(), String> {
    println!("🔄 Installing update...");

    // Backup current exe
    if backup_exe.exists() {
        fs::remove_file(backup_exe).ok();
    }
    fs::rename(current_exe, backup_exe)
        .map_err(|e| format!("Failed to backup current exe: {}", e))?;

    // Move new exe to current location
    fs::rename(temp_exe, current_exe)
        .map_err(|e| format!("Failed to replace exe: {}", e))?;

    println!("✅ Update installed successfully!");
    println!("🔄 Restarting...");

    // Restart the app
    restart_app(current_exe)?;

    Ok(())
}

/// Restart the application
fn restart_app(exe_path: &PathBuf) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new(exe_path)
            .spawn()
            .map_err(|e| format!("Failed to restart app: {}", e))?;
        std::process::exit(0);
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("Please restart the app manually.");
        std::process::exit(0);
    }
}

/// Get current version string
pub fn current_version() -> &'static str {
    CURRENT_VERSION
}

/// Print update check result and auto-download if available
#[allow(dead_code)]
pub fn print_update_status() {
    match check_for_update() {
        Ok(Some(update)) => {
            println!(
                "\n🔄 Update available: v{} → v{}",
                CURRENT_VERSION, update.version
            );
            println!("   Download URL: {}", update.download_url);
            println!();

            // Auto-download
            println!("   Auto-downloading update...");

            if let Err(e) = download_and_install(&update) {
                println!("❌ Update failed: {}", e);
                println!("   Please download manually from:");
                println!("   {}", update.download_url);
            }
        }
        Ok(None) => {
            log::info!("✅ Up to date (v{})", CURRENT_VERSION);
        }
        Err(e) => {
            log::warn!("Could not check for updates: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_download_url() {
        let release = serde_json::json!({
            "assets": [
                {"name": "rust_teams-windows-x64.exe", "browser_download_url": "https://example.com/exe"},
                {"name": "rust_teams-windows-x64.zip", "browser_download_url": "https://example.com/zip"}
            ]
        });

        let url = find_download_url(&release, "0.1.0").unwrap();
        assert_eq!(url, "https://example.com/exe");
    }

    #[test]
    fn test_find_download_url_fallback() {
        let release = serde_json::json!({
            "assets": []
        });

        let url = find_download_url(&release, "0.1.0").unwrap();
        assert!(url.contains("rust_teams-windows-x64.exe"));
    }
}
