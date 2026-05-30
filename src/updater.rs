//! Auto-update module — checks GitHub Releases for new versions

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

/// Get current version string
pub fn current_version() -> &'static str {
    CURRENT_VERSION
}

/// Print update check result
pub fn print_update_status() {
    match check_for_update() {
        Ok(Some(update)) => {
            println!(
                "\n🔄 Update available: v{} → v{}",
                CURRENT_VERSION, update.version
            );
            println!("   Download: {}", update.download_url);
            println!("   Run 'cargo install --path .' or download from releases to update.\n");
        }
        Ok(None) => {
            log::info!("✅ Up to date (v{})", CURRENT_VERSION);
        }
        Err(e) => {
            log::warn!("Could not check for updates: {}", e);
        }
    }
}
