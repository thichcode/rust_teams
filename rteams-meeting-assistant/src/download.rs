use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use anyhow::Result;

const BIN_REPO: &str = "ggerganov/whisper.cpp";
const MODEL_REPO: &str = "ggerganov/whisper.cpp";
const MODEL_FILE: &str = "ggml-tiny.en.bin";

/// Fetch the latest release tag from GitHub API (e.g. "v1.7.4").
fn latest_release_tag() -> Result<String> {
    let url = format!("https://api.github.com/repos/{BIN_REPO}/releases/latest");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("rust_teams-whisper")
        .build()?;
    let resp = client.get(&url).send()?;
    let json: serde_json::Value = resp.json()?;
    json["tag_name"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Could not parse latest release tag"))
}

/// Build download URLs for the latest whisper.cpp release.
fn binary_url(tag: &str) -> String {
    format!("https://github.com/{BIN_REPO}/releases/download/{tag}/whisper-bin-x64.zip")
}

fn model_url() -> String {
    format!("https://huggingface.co/{MODEL_REPO}/resolve/main/{MODEL_FILE}")
}

pub struct WhisperDownloader {
    data_dir: PathBuf,
}

impl WhisperDownloader {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn whisper_dir(&self) -> PathBuf {
        self.data_dir.join("whisper")
    }

    pub fn bin_path(&self) -> PathBuf {
        self.whisper_dir().join("main.exe")
    }

    pub fn model_path(&self) -> PathBuf {
        self.whisper_dir().join(MODEL_FILE)
    }

    #[allow(dead_code)]
    pub fn needs_download(&self) -> bool {
        !self.bin_path().exists() || !self.model_path().exists()
    }

    pub fn ensure_downloaded(&self, progress: &mpsc::Sender<String>) -> Result<()> {
        let dir = self.whisper_dir();
        fs::create_dir_all(&dir)?;

        if !self.model_path().exists() {
            let _ = progress.send("Downloading whisper model (75 MB)...".into());
            download_file(&model_url(), &self.model_path())?;
            let _ = progress.send("Model downloaded".into());
        }

        if !self.bin_path().exists() {
            let _ = progress.send("Downloading whisper.cpp binary...".into());
            let tag = latest_release_tag().unwrap_or_else(|_| "v1.7.4".to_string());
            let bin_url = binary_url(&tag);
            let zip_path = dir.join("whisper-bin-x64.zip");
            download_file(&bin_url, &zip_path)?;
            extract_bin(&zip_path, &self.bin_path())?;
            let _ = fs::remove_file(&zip_path);
            let _ = progress.send(format!("Binary extracted (release {tag})"));
        }

        Ok(())
    }
}

fn download_file(url: &str, dest: &Path) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .user_agent("rust_teams-whisper")
            .build()?;

        for attempt in 1..=3 {
            let resp = client.get(url).send().await;
            match resp {
                Ok(resp) => {
                    let bytes = resp.bytes().await?;
                    fs::write(dest, bytes)?;
                    return Ok(());
                }
                Err(e) => {
                    if attempt < 3 {
                        log::warn!("Download attempt {attempt}/3 failed: {e}");
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    } else {
                        anyhow::bail!("Failed to download after 3 attempts: {e}");
                    }
                }
            }
        }
        Ok(())
    })
}

fn extract_bin(zip_path: &Path, out_path: &Path) -> Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().replace('\\', "/");
        if name.ends_with("main.exe") {
            let mut out = fs::File::create(out_path)?;
            std::io::copy(&mut entry, &mut out)?;
            out.sync_all()?;
            return Ok(());
        }
    }

    anyhow::bail!("Could not find main.exe in zip archive")
}
