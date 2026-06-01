//! Auto-download whisper.cpp binary + model on startup.
//! Downloads from GitHub releases (binary) and HuggingFace (model).

#![allow(dead_code)]

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;

const WHISPER_VERSION: &str = "v1.7.4";
const BIN_URL: &str =
    "https://github.com/ggerganov/whisper.cpp/releases/download/v1.7.4/whisper-bin-x64.zip";
const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin";

/// Manages download + extraction of whisper.cpp CLI + model.
pub struct WhisperDownloader {
    data_dir: PathBuf,
    bin_url: String,
    model_url: String,
    /// Target filename of the binary inside the zip (e.g. "main.exe")
    bin_in_zip: &'static str,
}

impl WhisperDownloader {
    /// Create a new downloader that stores files under `data_dir/whisper/`.
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            bin_url: BIN_URL.to_string(),
            model_url: MODEL_URL.to_string(),
            bin_in_zip: "main.exe",
        }
    }

    /// Override the download URLs (useful for testing / mirrors).
    #[allow(dead_code)]
    pub fn with_urls(mut self, bin_url: &str, model_url: &str) -> Self {
        self.bin_url = bin_url.to_string();
        self.model_url = model_url.to_string();
        Self { ..self }
    }

    /// Directory where whisper files are stored.
    fn whisper_dir(&self) -> PathBuf {
        self.data_dir.join("whisper")
    }

    /// Path to the extracted `main.exe`.
    pub fn bin_path(&self) -> PathBuf {
        self.whisper_dir().join("main.exe")
    }

    /// Path to the downloaded ggml model.
    pub fn model_path(&self) -> PathBuf {
        self.whisper_dir().join("ggml-tiny.en.bin")
    }

    /// Whether a download is needed.
    pub fn needs_download(&self) -> bool {
        !self.bin_path().exists() || !self.model_path().exists()
    }

    /// Run the full download + extraction. Idempotent — skips existing files.
    pub fn ensure_downloaded(&self) -> Result<()> {
        let dir = self.whisper_dir();
        fs::create_dir_all(&dir)?;

        if !self.model_path().exists() {
            log::info!("[Whisper] Downloading model ({}MB)…", 75);
            download_file(&self.model_url, &self.model_path())?;
            log::info!("[Whisper] Model downloaded ✓");
        } else {
            log::info!("[Whisper] Model already exists");
        }

        if !self.bin_path().exists() {
            log::info!("[Whisper] Downloading whisper.cpp binary…");
            let zip_path = dir.join("whisper-bin-x64.zip");
            download_file(&self.bin_url, &zip_path)?;
            extract_bin(&zip_path, &dir, self.bin_in_zip, &self.bin_path())?;
            let _ = fs::remove_file(&zip_path);
            log::info!("[Whisper] Binary extracted ✓");
        } else {
            log::info!("[Whisper] Binary already exists");
        }

        Ok(())
    }
}

/// Download a file from URL to local path (blocking, with retry).
fn download_file(url: &str, dest: &Path) -> Result<()> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(300))
        .user_agent("rust_teams-whisper")
        .build()?;

    for attempt in 1..=3 {
        let resp = client.get(url).send().map_err(|e| {
            if attempt < 3 {
                log::warn!("[Whisper] Download attempt {}/3 failed: {}", attempt, e);
            }
            e
        });

        if let Ok(mut resp) = resp {
            let mut file = fs::File::create(dest)?;
            let mut buf = [0u8; 65536];
            let mut total = 0u64;
            loop {
                let n = resp.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                file.write_all(&buf[..n])?;
                total += n as u64;
            }
            file.sync_all()?;
            log::info!("[Whisper] Downloaded {} bytes", total);
            return Ok(());
        } else if attempt < 3 {
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    anyhow::bail!("Failed to download {} after 3 attempts", url)
}

/// Extract a single file from a zip archive.
fn extract_bin(zip_path: &Path, _dest_dir: &Path, target_name: &str, out_path: &Path) -> Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().replace('\\', "/");

        // Find the target binary anywhere in the zip tree
        if name.ends_with(target_name) || name.ends_with(&format!("/{}", target_name)) {
            let mut out = fs::File::create(out_path)?;
            std::io::copy(&mut entry, &mut out)?;
            out.sync_all()?;
            log::info!("[Whisper] Extracted {}", name);
            return Ok(());
        }
    }

    anyhow::bail!(
        "Could not find '{}' inside zip archive ({} entries)",
        target_name,
        archive.len()
    )
}
