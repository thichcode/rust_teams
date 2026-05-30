//! Auto-update module — checks GitHub Releases and downloads updates

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Duration;

const REPO: &str = "thichcode/rust_teams";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
        .timeout(Duration::from_secs(10))
        .user_agent("rust_teams")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&url)
        .send()
        .map_err(|e| format!("Failed to check for updates: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API returned status: {}", response.status()));
    }

    let release: serde_json::Value = response
        .json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let tag_name = release["tag_name"]
        .as_str()
        .ok_or("Missing tag_name")?
        .trim_start_matches('v')
        .to_string();

    let current = semver::Version::parse(CURRENT_VERSION)
        .map_err(|e| format!("Invalid current version: {}", e))?;
    let latest =
        semver::Version::parse(&tag_name).map_err(|e| format!("Invalid latest version: {}", e))?;

    if latest <= current {
        log::info!("Already on latest version (v{})", CURRENT_VERSION);
        return Ok(None);
    }

    log::info!("Update available: v{} → v{}", CURRENT_VERSION, tag_name);

    // Find the exe asset
    let download_url = release["assets"]
        .as_array()
        .and_then(|assets| {
            assets.iter().find_map(|asset| {
                let name = asset["name"].as_str()?;
                if name.ends_with(".exe") && name.contains("x64") {
                    Some(asset["browser_download_url"].as_str()?.to_string())
                } else {
                    None
                }
            })
        })
        .unwrap_or_else(|| {
            format!(
                "https://github.com/{}/releases/download/v{}/rust_teams.exe",
                REPO, tag_name
            )
        });

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

/// Download update to a temp file, then replace current exe and restart
pub fn download_and_install(update: &UpdateInfo) -> Result<(), String> {
    println!("⏳ Downloading v{}...", update.version);

    // Create HTTP client with longer timeout for download
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(300)) // 5 minutes timeout
        .user_agent("rust_teams")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Download the new exe
    let mut response = client
        .get(&update.download_url)
        .send()
        .map_err(|e| format!("Failed to download update: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    // Get current exe path
    let current_exe = std::env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;
    let exe_dir = current_exe.parent().ok_or("Failed to get exe directory")?;

    // Create temp file path
    let temp_exe = exe_dir.join("rust_teams.exe.tmp");
    let backup_exe = exe_dir.join("rust_teams.exe.bak");

    // Download to temp file
    let mut file = fs::File::create(&temp_exe)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    let mut total_bytes = 0u64;
    let mut buffer = vec![0u8; 8192];

    loop {
        let bytes_read = response.read(&mut buffer).map_err(|e| format!("Read error: {}", e))?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])
            .map_err(|e| format!("Write error: {}", e))?;
        total_bytes += bytes_read as u64;

        // Progress indicator
        print!("\r   Downloaded: {} KB", total_bytes / 1024);
        std::io::stdout().flush().ok();
    }
    println!(); // New line after progress

    // Flush and close the file
    file.flush().map_err(|e| format!("Flush error: {}", e))?;
    drop(file);

    println!("✅ Download complete ({} KB)", total_bytes / 1024);
    println!("🔄 Installing update...");

    // Backup current exe
    if backup_exe.exists() {
        fs::remove_file(&backup_exe).ok();
    }
    fs::rename(&current_exe, &backup_exe).map_err(|e| format!("Failed to backup current exe: {}", e))?;

    // Move new exe to current location
    fs::rename(&temp_exe, &current_exe).map_err(|e| format!("Failed to replace exe: {}", e))?;

    println!("✅ Update installed successfully!");
    println!("🔄 Restarting...");

    // Restart the app
    restart_app(&current_exe)?;

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
        // On non-Windows, just exit and let user restart manually
        println!("Please restart the app manually.");
        std::process::exit(0);
    }
}

/// Get current version string
pub fn current_version() -> &'static str {
    CURRENT_VERSION
}

/// Print update check result and auto-download if available
pub fn print_update_status() {
    match check_for_update() {
        Ok(Some(update)) => {
            println!(
                "\n🔄 Update available: v{} → v{}",
                CURRENT_VERSION, update.version
            );
            println!("   Download URL: {}", update.download_url);
            println!();

            // Ask user if they want to update
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
