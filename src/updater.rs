//! Auto-update module — checks GitHub Releases and downloads updates

use sha2::{Digest, Sha256};
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
    /// URL to the `.sha256` checksum file for `download_url`, if published.
    pub checksum_url: Option<String>,
    pub body: String,
}

/// Check if a newer version is available on GitHub Releases (main app only, v* tags)
pub fn check_for_update() -> Result<Option<UpdateInfo>, String> {
    // Use releases list (not /latest) to filter for main app tags (v* only)
    // Mini app uses rteams-meeting-assistant-v* tags which would be "latest" but wrong app
    let url = format!("https://api.github.com/repos/{}/releases?per_page=10", REPO);

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
        if let Some(retry_after) = response.headers().get("Retry-After")
            && let Ok(seconds) = retry_after.to_str().unwrap_or("60").parse::<u64>()
        {
            return Err(format!(
                "GitHub API rate limited. Try again in {} seconds",
                seconds
            ));
        }
        return Err("GitHub API rate limited. Try again later.".to_string());
    }

    if !response.status().is_success() {
        return Err(format!("GitHub API returned status: {}", response.status()));
    }

    let releases: serde_json::Value = response
        .json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    // Find first release with main app tag (vX.Y.Z)
    let release = releases
        .as_array()
        .ok_or("Invalid releases response")?
        .iter()
        .find(|r| {
            r["tag_name"]
                .as_str()
                .map(|t| {
                    t.starts_with('v') && t[1..].chars().next().is_some_and(|c| c.is_ascii_digit())
                })
                .unwrap_or(false)
        })
        .ok_or("No main app release found")?;

    let tag_name = release["tag_name"]
        .as_str()
        .ok_or("Missing tag_name")?
        .to_string();
    let tag_name = tag_name.trim_start_matches('v').to_string();

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
    let download_url = find_download_url(release, &tag_name)?;
    let checksum_url = find_checksum_url(release, &tag_name);

    let body = release["body"]
        .as_str()
        .unwrap_or("No release notes")
        .to_string();

    Ok(Some(UpdateInfo {
        version: tag_name,
        download_url,
        checksum_url,
        body,
    }))
}

/// Filename of the auto-update asset for the current platform.
fn platform_asset_name() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "rust_teams-linux-x64.tar.gz"
    }
    #[cfg(target_os = "windows")]
    {
        "rust_teams-windows-x64.exe"
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        "rust_teams-windows-x64.exe"
    }
}

/// Find the download URL from release assets (platform-aware)
fn find_download_url(release: &serde_json::Value, tag: &str) -> Result<String, String> {
    let asset_name = platform_asset_name();

    if let Some(assets) = release["assets"].as_array() {
        // Priority 1: exact platform asset name
        for asset in assets {
            if let Some(name) = asset["name"].as_str()
                && name == asset_name
            {
                return asset["browser_download_url"]
                    .as_str()
                    .map(|s| s.to_string())
                    .ok_or("Missing download URL".to_string());
            }
        }

        // Priority 2: any platform-appropriate asset
        let candidates: &[&str] = platform_candidates();
        for asset in assets {
            if let Some(name) = asset["name"].as_str()
                && candidates.iter().any(|c| name.ends_with(c))
            {
                return asset["browser_download_url"]
                    .as_str()
                    .map(|s| s.to_string())
                    .ok_or("Missing download URL".to_string());
            }
        }
    }

    // Fallback URL
    Ok(format!(
        "https://github.com/{}/releases/download/v{}/{}",
        REPO, tag, asset_name
    ))
}

/// Candidate filename suffixes per platform, for loose asset matching.
fn platform_candidates() -> &'static [&'static str] {
    #[cfg(target_os = "linux")]
    {
        &["tar.gz", ".deb"]
    }
    #[cfg(target_os = "windows")]
    {
        &[".exe"]
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        &[".exe"]
    }
}

/// Find the `.sha256` checksum asset matching the platform download, if published.
fn find_checksum_url(release: &serde_json::Value, tag: &str) -> Option<String> {
    let checksum_name = format!("{}.sha256", platform_asset_name());
    if let Some(assets) = release["assets"].as_array() {
        for asset in assets {
            if let Some(name) = asset["name"].as_str()
                && name == checksum_name
            {
                return asset["browser_download_url"]
                    .as_str()
                    .map(|s| s.to_string());
            }
        }
    }

    // Fallback URL (only valid if the release actually publishes this asset)
    Some(format!(
        "https://github.com/{}/releases/download/v{}/{}",
        REPO, tag, checksum_name
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

    // Create temp and backup paths (platform-neutral names)
    let temp_exe = exe_dir.join("rust_teams.tmp");
    let backup_exe = exe_dir.join("rust_teams.bak");

    // Clean up any leftover temp files
    if temp_exe.exists() {
        fs::remove_file(&temp_exe).ok();
    }

    // Fetch expected SHA256 checksum before installing anything (best-effort:
    // older releases may not publish a checksum asset yet).
    let expected_checksum =
        update
            .checksum_url
            .as_deref()
            .and_then(|url| match fetch_checksum(&client, url) {
                Ok(sum) => Some(sum),
                Err(e) => {
                    log::warn!(
                        "Could not fetch checksum ({}), skipping checksum verification: {}",
                        url,
                        e
                    );
                    None
                }
            });

    // Download with retry
    let mut last_error = String::new();
    for attempt in 1..=MAX_RETRIES {
        println!("   Attempt {}/{}...", attempt, MAX_RETRIES);

        match download_file(&client, &update.download_url, &temp_exe) {
            Ok(size) => {
                println!("✅ Download complete ({} KB)", size / 1024);

                // Validate downloaded file (size, PE header, and checksum if available)
                if let Err(e) = validate_download(&temp_exe, expected_checksum.as_deref()) {
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
                    println!(
                        "⚠️  Download failed, retrying in {}s...",
                        RETRY_DELAY_MS / 1000
                    );
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

/// Download and parse a `.sha256` checksum file (format: `<hex>  <filename>`).
/// Returns the lowercase hex digest.
fn fetch_checksum(client: &reqwest::blocking::Client, url: &str) -> Result<String, String> {
    let response = client
        .get(url)
        .send()
        .map_err(|e| format!("Failed to fetch checksum: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Checksum fetch returned status: {}",
            response.status()
        ));
    }

    let text = response
        .text()
        .map_err(|e| format!("Failed to read checksum body: {}", e))?;

    let hex = text
        .split_whitespace()
        .next()
        .ok_or("Empty checksum file")?
        .to_lowercase();

    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("Malformed SHA256 checksum: {}", hex));
    }

    Ok(hex)
}

/// Compute the SHA256 checksum of a file, as a lowercase hex string.
fn compute_sha256(path: &PathBuf) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 65536];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|e| format!("Read error while hashing: {}", e))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
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
        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));
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

    file.flush().map_err(|e| format!("Flush error: {}", e))?;
    drop(file);

    Ok(total_bytes)
}

/// Validate downloaded file: size, PE header, and (if available) SHA256 checksum
/// against the value published alongside the release. Refuses to install if the
/// checksum was fetched successfully but does not match.
fn validate_download(path: &PathBuf, expected_sha256: Option<&str>) -> Result<(), String> {
    let metadata =
        fs::metadata(path).map_err(|e| format!("Failed to read file metadata: {}", e))?;

    // Check file size (should be at least 1MB for a valid update)
    if metadata.len() < 1024 * 1024 {
        return Err(format!(
            "File too small: {} bytes (expected at least 1MB)",
            metadata.len()
        ));
    }

    let mut file = fs::File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut header = [0u8; 2];
    file.read_exact(&mut header)
        .map_err(|e| format!("Failed to read header: {}", e))?;
    drop(file);

    // Verify the platform binary magic header.
    #[cfg(target_os = "windows")]
    {
        // DOS/PE executable ("MZ")
        if header[0] != b'M' || header[1] != b'Z' {
            return Err("Not a valid Windows executable (missing MZ header)".to_string());
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Linux update is a gzip'd tarball (magic: 0x1f 0x8b)
        if header[0] != 0x1f || header[1] != 0x8b {
            return Err("Not a valid Linux update (missing gzip magic)".to_string());
        }
    }

    // Verify SHA256 checksum against the published value, when available.
    if let Some(expected) = expected_sha256 {
        let actual = compute_sha256(path)?;
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(format!(
                "Checksum mismatch: expected {}, got {}",
                expected, actual
            ));
        }
        println!("✅ Checksum verified (SHA256)");
    } else {
        log::warn!("Installing update without checksum verification (no checksum published)");
    }

    Ok(())
}

/// Install the update by replacing the current executable, or by installing
/// a fresh copy into the user's local bin dir when the current location is
/// not writable (e.g. `/usr/bin` from a `.deb` install).
fn install_update(
    temp_path: &PathBuf,
    current_exe: &PathBuf,
    backup_path: &PathBuf,
) -> Result<(), String> {
    println!("🔄 Installing update...");

    #[cfg(target_os = "linux")]
    {
        // The downloaded artifact is a gzip'd tarball. Extract the binary,
        // then try to replace it in place. If the current location is not
        // writable (e.g. `/usr/bin` from a `.deb`), fall back to a copy in
        // the user's `~/.local/bin` and point the launcher there.
        let extracted = extract_tarball(temp_path)?;

        if let Ok(target) = replace_in_place(&extracted, current_exe, backup_path) {
            println!("✅ Update installed successfully!");
            println!("🔄 Restarting...");
            restart_app(&target)?;
            return Ok(());
        }

        let target = install_user_bin(&extracted)?;
        println!("✅ Update installed successfully!");
        println!(
            "ℹ️  Installed to {} (current location {} is not writable)",
            target.display(),
            current_exe.display()
        );
        println!(
            "   Old install at {} can be removed with: sudo apt remove rust-teams",
            current_exe.display()
        );
        println!("🔄 Restarting...");
        restart_app(&target)?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        // Backup current exe
        if backup_path.exists() {
            fs::remove_file(backup_path).ok();
        }
        fs::rename(current_exe, backup_path)
            .map_err(|e| format!("Failed to backup current exe: {}", e))?;

        // Move new exe to current location
        fs::rename(temp_path, current_exe).map_err(|e| format!("Failed to replace exe: {}", e))?;

        println!("✅ Update installed successfully!");
        println!("🔄 Restarting...");

        // Restart the app
        restart_app(current_exe)?;
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (temp_path, current_exe, backup_path);
        Err("Auto-update is not supported on this platform".to_string())
    }
}

/// Try to replace the running executable in place with the new binary.
///
/// Returns the target path on success; returns `Err` if the current
/// directory is not writable, in which case the caller falls back to a
/// user-local install.
#[cfg(target_os = "linux")]
fn replace_in_place(
    new_binary: &PathBuf,
    current_exe: &PathBuf,
    backup_path: &PathBuf,
) -> Result<PathBuf, String> {
    if backup_path.exists() {
        fs::remove_file(backup_path).ok();
    }
    fs::rename(current_exe, backup_path)
        .map_err(|e| format!("Cannot update in place ({}), preparing user install", e))?;
    if fs::rename(new_binary, current_exe).is_err() {
        fs::copy(new_binary, current_exe)
            .map_err(|e| format!("Failed to replace binary in place: {}", e))?;
    }
    make_executable(current_exe)?;
    Ok(current_exe.to_path_buf())
}

/// Install a copy of the new binary into `~/.local/bin/rust-teams` and
/// point a `~/.local/share/applications` launcher entry at it.
#[cfg(target_os = "linux")]
fn install_user_bin(new_binary: &PathBuf) -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("HOME not set")?;
    let home = std::path::PathBuf::from(&home);
    let bin_dir = home.join(".local").join("bin");
    fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("Failed to create {}: {}", bin_dir.display(), e))?;

    let target = bin_dir.join("rust-teams");
    fs::copy(new_binary, &target)
        .map_err(|e| format!("Failed to copy binary to {}: {}", target.display(), e))?;
    make_executable(&target)?;

    install_user_desktop_entry(&home)?;
    Ok(target)
}

/// Write a `.desktop` launcher entry so the freshly installed binary shows
/// in the app launcher.
#[cfg(target_os = "linux")]
fn install_user_desktop_entry(home: &std::path::Path) -> Result<(), String> {
    let applications = home.join(".local").join("share").join("applications");
    fs::create_dir_all(&applications)
        .map_err(|e| format!("Failed to create {}: {}", applications.display(), e))?;

    let entry = [
        "[Desktop Entry]",
        "Type=Application",
        "Name=R Teams",
        format!(
            "Exec={}",
            home.join(".local").join("bin").join("rust-teams").display()
        )
        .as_str(),
        "Terminal=false",
        "Categories=Network;InstantMessaging;",
        "StartupWMClass=rust-teams",
    ]
    .join("\n");

    fs::write(applications.join("rust-teams.desktop"), entry)
        .map_err(|e| format!("Failed to write desktop entry: {}", e))?;
    Ok(())
}

/// Extract a gzip'd tarball to a temp dir and return the path to the binary.
#[cfg(target_os = "linux")]
fn extract_tarball(tarball: &PathBuf) -> Result<PathBuf, String> {
    use std::process::Command;
    let extract_dir =
        std::env::temp_dir().join(format!("rust-teams-extract-{}", std::process::id()));
    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir).ok();
    }
    fs::create_dir_all(&extract_dir).map_err(|e| format!("Failed to create extract dir: {}", e))?;

    let status = Command::new("tar")
        .arg("-xzf")
        .arg(tarball)
        .arg("-C")
        .arg(&extract_dir)
        .status()
        .map_err(|e| format!("Failed to run tar: {}", e))?;
    if !status.success() {
        return Err("Failed to extract update archive".to_string());
    }

    // The archive contains a single top-level binary named rust_teams-linux-x64
    let binary = extract_dir.join("rust_teams-linux-x64");
    if !binary.is_file() {
        return Err(format!(
            "Extracted archive did not contain expected binary at {}",
            binary.display()
        ));
    }
    Ok(binary)
}

/// Mark a file as executable (POSIX only).
#[cfg(target_os = "linux")]
fn make_executable(path: &PathBuf) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)
        .map_err(|e| format!("Failed to read metadata: {}", e))?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).map_err(|e| format!("Failed to chmod: {}", e))
}

/// Restart the application from the given executable path.
fn restart_app(exe_path: &PathBuf) -> Result<(), String> {
    use std::process::Command;

    Command::new(exe_path)
        .spawn()
        .map_err(|e| format!("Failed to restart app: {}", e))?;
    std::process::exit(0);
}

/// Cached result of an update check — avoids duplicate API calls.
#[derive(Debug, Clone)]
pub enum UpdateCheck {
    Available(UpdateInfo),
    Latest,
    Error(String),
}

impl UpdateCheck {
    pub fn version_info(&self) -> String {
        match self {
            Self::Available(update) => format!(
                "📦 Version: v{} (update available: v{})",
                CURRENT_VERSION, update.version
            ),
            Self::Latest => format!("📦 Version: v{} (latest)", CURRENT_VERSION),
            Self::Error(e) => format!("📦 Version: v{} (check failed: {e})", CURRENT_VERSION),
        }
    }

    #[allow(dead_code)]
    pub fn update(&self) -> Option<&UpdateInfo> {
        match self {
            Self::Available(update) => Some(update),
            _ => None,
        }
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
                {"name": "rust_teams-linux-x64.tar.gz", "browser_download_url": "https://example.com/tar.gz"},
                {"name": "rust_teams-windows-x64.zip", "browser_download_url": "https://example.com/zip"}
            ]
        });

        let url = find_download_url(&release, "0.1.0").unwrap();
        #[cfg(target_os = "windows")]
        assert_eq!(url, "https://example.com/exe");
        #[cfg(target_os = "linux")]
        assert_eq!(url, "https://example.com/tar.gz");
    }

    #[test]
    fn test_find_download_url_fallback() {
        let release = serde_json::json!({ "assets": [] });
        let url = find_download_url(&release, "0.1.0").unwrap();
        assert!(url.contains(platform_asset_name()));
    }

    #[test]
    fn test_find_checksum_url_from_assets() {
        let release = serde_json::json!({
            "assets": [
                {"name": "rust_teams-windows-x64.exe.sha256", "browser_download_url": "https://example.com/exe.sha256"},
                {"name": "rust_teams-linux-x64.tar.gz.sha256", "browser_download_url": "https://example.com/tar.gz.sha256"}
            ]
        });

        let url = find_checksum_url(&release, "0.1.0").unwrap();
        #[cfg(target_os = "windows")]
        assert_eq!(url, "https://example.com/exe.sha256");
        #[cfg(target_os = "linux")]
        assert_eq!(url, "https://example.com/tar.gz.sha256");
    }

    #[test]
    fn test_find_checksum_url_fallback() {
        let release = serde_json::json!({ "assets": [] });
        let url = find_checksum_url(&release, "0.1.0").unwrap();
        assert!(url.contains(platform_asset_name()));
        assert!(url.ends_with(".sha256"));
    }

    #[test]
    fn test_compute_sha256() {
        let dir = std::env::temp_dir();
        let path = dir.join("rteams_test_checksum.bin");
        fs::write(&path, b"hello world").unwrap();

        let hash = compute_sha256(&path).unwrap();
        // SHA256("hello world")
        // Verified: echo -n "hello world" | sha256sum
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );

        fs::remove_file(&path).ok();
    }

    /// Build a file whose first two bytes pass the current platform's
    /// magic-header check and is &gt; 1MB in size.
    fn platform_valid_download_file(path: &PathBuf) {
        let mut data = vec![0u8; 1024 * 1024 + 2];
        #[cfg(target_os = "windows")]
        {
            data[0] = b'M';
            data[1] = b'Z';
        }
        #[cfg(target_os = "linux")]
        {
            data[0] = 0x1f;
            data[1] = 0x8b;
        }
        fs::write(path, &data).unwrap();
    }

    #[test]
    fn test_validate_download_checksum_mismatch() {
        let dir = std::env::temp_dir();
        let path = dir.join("rteams_test_validate_mismatch");
        platform_valid_download_file(&path);

        let wrong_checksum = "0".repeat(64);
        let result = validate_download(&path, Some(&wrong_checksum));
        assert!(result.is_err());

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_validate_download_checksum_match() {
        let dir = std::env::temp_dir();
        let path = dir.join("rteams_test_validate_match");
        platform_valid_download_file(&path);

        let expected = compute_sha256(&path).unwrap();
        let result = validate_download(&path, Some(&expected));
        assert!(result.is_ok());

        fs::remove_file(&path).ok();
    }
}
