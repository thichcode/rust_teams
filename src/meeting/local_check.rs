//! Local-mode readiness check + wizard model catalog.
//!
//! Verifies that Ollama server is reachable + has the chosen model,
//! and that the whisper.cpp binary + model files exist on disk.

#![allow(dead_code)]

use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Status of a single local provider (Ollama or whisper.cpp).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ProviderStatus {
    Ready { model: String },
    NotInstalled { install_url: String, hint: String },
    NotRunning { endpoint: String, install_hint: String },
    ModelMissing { endpoint: String, model: String, install_hint: String },
    WrongPath { expected: String, actual: String },
}

/// Combined readiness for the 3 providers. Note: `ollama` covers both
/// translator and suggester since they share the same server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalReadiness {
    pub ollama: ProviderStatus,
    pub whisper: ProviderStatus,
}

/// One selectable model in the wizard dropdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelOption {
    pub id: String,
    pub label: String,
    pub size_mb: u32,
    pub recommended: bool,
    pub install_hint: String,
}

/// Catalog of models shown in the 3 wizard steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardOptions {
    pub stt: Vec<ModelOption>,
    pub translator: Vec<ModelOption>,
    pub suggester: Vec<ModelOption>,
    pub whisper_binary_path: String,
    pub ollama_endpoint: String,
}

/// User's wizard selections (sent from JS to Rust in `local_setup_apply`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalChoices {
    pub stt: SelectedModel,
    pub translator: SelectedModel,
    pub suggester: SelectedModel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedModel {
    pub id: String,
    pub path: Option<String>,
    pub endpoint: Option<String>,
}

/// A single installed model returned by Ollama's `/api/tags`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    #[serde(default)]
    models: Vec<OllamaModel>,
}

/// Lightweight Ollama HTTP client. Only needs `/api/tags` for now.
pub struct OllamaClient {
    endpoint: String,
    http: Client,
}

impl OllamaClient {
    pub fn new(endpoint: &str) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("reqwest client");
        Self { endpoint: endpoint.trim_end_matches('/').to_string(), http }
    }

    /// GET /api/tags — list installed models.
    pub async fn list_models(&self) -> Result<Vec<OllamaModel>> {
        let url = format!("{}/api/tags", self.endpoint);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .with_context(|| format!("GET {url} failed (is Ollama running?)"))?;
        if !resp.status().is_success() {
            return Err(anyhow!("Ollama returned HTTP {}", resp.status()));
        }
        let body: OllamaTagsResponse = resp
            .json()
            .await
            .context("Ollama returned non-JSON response")?;
        Ok(body.models)
    }

    /// Check whether a specific model id is in the installed list.
    pub async fn has_model(&self, model_id: &str) -> Result<bool> {
        let models = self.list_models().await?;
        Ok(models.iter().any(|m| m.name == model_id))
    }
}

/// Check the whisper.cpp binary + model file status.
pub fn whisper_status(
    binary: &Path,
    model_file: &Path,
    model_id: &str,
) -> ProviderStatus {
    let bin_ok = binary.is_file();
    let mdl_ok = model_file.is_file();
    match (bin_ok, mdl_ok) {
        (true, true) => ProviderStatus::Ready {
            model: model_id.to_string(),
        },
        _ => ProviderStatus::NotInstalled {
            install_url: "https://github.com/ggerganov/whisper.cpp/releases".to_string(),
            hint: format!(
                "Download whisper.cpp release + '{}' model. R Teams can auto-download — click 'Download now' in the wizard.",
                model_id
            ),
        },
    }
}

use crate::meeting::realtime_config::RealtimeTranslateConfig;

/// Check readiness of both local providers using the current config.
/// Ollama readiness is shared by translator + suggester.
pub async fn check_local_readiness(cfg: &RealtimeTranslateConfig) -> LocalReadiness {
    let ollama = check_ollama_readiness(cfg).await;
    let whisper = check_whisper_readiness(cfg);
    LocalReadiness { ollama, whisper }
}

async fn check_ollama_readiness(cfg: &RealtimeTranslateConfig) -> ProviderStatus {
    let endpoint = if cfg.translator.api_url.is_empty() {
        "http://localhost:11434".to_string()
    } else {
        cfg.translator.api_url.clone()
    };
    let client = OllamaClient::new(&endpoint);
    let models = match client.list_models().await {
        Ok(m) => m,
        Err(_) => {
            return ProviderStatus::NotRunning {
                endpoint: endpoint.clone(),
                install_hint: "Install Ollama from https://ollama.com/download, then run: ollama serve".to_string(),
            };
        }
    };
    let model = if !cfg.translator.extra.is_empty() {
        cfg.translator.extra.clone()
    } else if !cfg.suggester.model.is_empty() {
        cfg.suggester.model.clone()
    } else {
        String::new()
    };
    if model.is_empty() {
        return ProviderStatus::Ready { model: "(none selected)".to_string() };
    }
    if models.iter().any(|m| m.name == model) {
        ProviderStatus::Ready { model }
    } else {
        ProviderStatus::ModelMissing {
            endpoint,
            model: model.clone(),
            install_hint: format!("Run: ollama pull {}", model),
        }
    }
}

fn check_whisper_readiness(cfg: &RealtimeTranslateConfig) -> ProviderStatus {
    let bin = Path::new(&cfg.stt.api_url);
    let mdl = Path::new(&cfg.stt.api_key);
    whisper_status(bin, mdl, &cfg.stt.model)
}

/// Build the static catalog of models the wizard offers. Does not
/// perform any I/O — Ollama-side options are added later by the wizard.
pub fn build_wizard_options(cfg: &RealtimeTranslateConfig) -> WizardOptions {
    let endpoint = if cfg.translator.api_url.is_empty() {
        "http://localhost:11434".to_string()
    } else {
        cfg.translator.api_url.clone()
    };
    let whisper_binary_path = if cfg.stt.api_url.is_empty() {
        default_whisper_binary_path()
    } else {
        cfg.stt.api_url.clone()
    };
    WizardOptions {
        stt: vec![
            ModelOption {
                id: "ggml-tiny.en".into(),
                label: "Tiny (English, ~75 MB, fastest)".into(),
                size_mb: 75,
                recommended: true,
                install_hint: "Auto-downloads to %APPDATA%\\RTeams\\whisper\\".into(),
            },
            ModelOption {
                id: "ggml-base.en".into(),
                label: "Base (English, ~150 MB, balanced)".into(),
                size_mb: 150,
                recommended: false,
                install_hint: "Auto-downloads to %APPDATA%\\RTeams\\whisper\\".into(),
            },
            ModelOption {
                id: "ggml-small".into(),
                label: "Small (multilingual, ~460 MB)".into(),
                size_mb: 460,
                recommended: false,
                install_hint: "Auto-downloads to %APPDATA%\\RTeams\\whisper\\".into(),
            },
        ],
        translator: vec![
            ModelOption {
                id: "qwen2.5:7b".into(),
                label: "Qwen 2.5 7B (~4.7 GB, multilingual, recommended)".into(),
                size_mb: 4700,
                recommended: true,
                install_hint: "ollama pull qwen2.5:7b".into(),
            },
            ModelOption {
                id: "gemma3:4b".into(),
                label: "Gemma 3 4B (~3.3 GB, fast on CPU)".into(),
                size_mb: 3300,
                recommended: false,
                install_hint: "ollama pull gemma3:4b".into(),
            },
            ModelOption {
                id: "llama3.2:3b".into(),
                label: "Llama 3.2 3B (~2.0 GB, smallest)".into(),
                size_mb: 2000,
                recommended: false,
                install_hint: "ollama pull llama3.2:3b".into(),
            },
        ],
        suggester: vec![
            ModelOption {
                id: "gemma3:4b".into(),
                label: "Gemma 3 4B (~3.3 GB, fast, recommended for short replies)".into(),
                size_mb: 3300,
                recommended: true,
                install_hint: "ollama pull gemma3:4b".into(),
            },
            ModelOption {
                id: "llama3.1:8b".into(),
                label: "Llama 3.1 8B (~4.9 GB, more natural phrasing)".into(),
                size_mb: 4900,
                recommended: false,
                install_hint: "ollama pull llama3.1:8b".into(),
            },
            ModelOption {
                id: "qwen2.5:7b".into(),
                label: "Qwen 2.5 7B (~4.7 GB, share with translator)".into(),
                size_mb: 4700,
                recommended: false,
                install_hint: "ollama pull qwen2.5:7b".into(),
            },
        ],
        whisper_binary_path,
        ollama_endpoint: endpoint,
    }
}

fn default_whisper_binary_path() -> String {
    if let Some(proj) = directories::ProjectDirs::from("com", "rust-teams", "app") {
        return proj.data_dir().join("whisper").join("main.exe").display().to_string();
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meeting::realtime_config::{LocalPreset, RealtimeTranslateConfig};

    // --- Whisper tests ---

    #[test]
    fn whisper_status_ready_when_both_paths_exist() {
        let tmp = std::env::temp_dir();
        let bin = tmp.join("rteams_test_w_main.exe");
        let mdl = tmp.join("rteams_test_w_model.bin");
        let _ = std::fs::write(&bin, b"x");
        let _ = std::fs::write(&mdl, b"x");

        let st = whisper_status(&bin, &mdl, "ggml-base.en");
        assert_eq!(st, ProviderStatus::Ready { model: "ggml-base.en".into() });

        let _ = std::fs::remove_file(&bin);
        let _ = std::fs::remove_file(&mdl);
    }

    #[test]
    fn whisper_status_not_installed_when_binary_missing() {
        let st = whisper_status(
            &Path::new("C:/nope.exe"),
            &Path::new("C:/nope.bin"),
            "ggml-base.en",
        );
        assert!(matches!(st, ProviderStatus::NotInstalled { .. }));
    }

    #[test]
    fn whisper_status_wrong_path_when_model_missing_only() {
        let tmp = std::env::temp_dir();
        let bin = tmp.join("rteams_test_w_bin2.exe");
        let _ = std::fs::write(&bin, b"x");
        let st = whisper_status(&bin, &Path::new("C:/nope.bin"), "ggml-base.en");
        assert!(matches!(st, ProviderStatus::NotInstalled { .. }));
        let _ = std::fs::remove_file(&bin);
    }

    // --- Wizard options tests ---

    #[test]
    fn wizard_options_returns_at_least_one_per_role() {
        let cfg = RealtimeTranslateConfig::default();
        let opts = build_wizard_options(&cfg);
        assert!(!opts.stt.is_empty());
        assert!(!opts.translator.is_empty());
        assert!(!opts.suggester.is_empty());
    }

    #[test]
    fn wizard_options_mark_recommended_models() {
        let cfg = RealtimeTranslateConfig::default();
        let opts = build_wizard_options(&cfg);
        assert!(opts.stt.iter().any(|m| m.recommended && m.id == "ggml-tiny.en"));
        assert!(opts.translator.iter().any(|m| m.recommended && m.id == "qwen2.5:7b"));
        assert!(opts.suggester.iter().any(|m| m.recommended && m.id == "gemma3:4b"));
    }

    // --- LocalPreset integration tests ---

    #[test]
    fn apply_local_preset_integration() {
        let mut cfg = RealtimeTranslateConfig::default();
        assert_eq!(cfg.stt.provider_type, "openai");
        let preset = LocalPreset {
            stt_model: "ggml-base.en".into(),
            translator_model: "qwen2.5:7b".into(),
            suggester_model: "gemma3:4b".into(),
            ollama_endpoint: "http://localhost:11434".into(),
            whisper_binary: "C:/whisper/main.exe".into(),
            whisper_model: "C:/whisper/model.bin".into(),
            last_checked: None,
        };
        cfg.apply_local_preset(&preset);
        assert_eq!(cfg.stt.provider_type, "local");
        assert_eq!(cfg.stt.model, "ggml-base.en");
        assert_eq!(cfg.translator.provider_type, "ollama");
        assert_eq!(cfg.translator.api_url, "http://localhost:11434");
        assert_eq!(cfg.suggester.provider_type, "ollama");
        assert_eq!(cfg.suggester.model, "gemma3:4b");
        // preserved
        cfg.target_lang = "ja".into();
        cfg.suggestion_count = 5;
        assert_eq!(cfg.target_lang, "ja");
        assert_eq!(cfg.suggestion_count, 5);
    }

    #[test]
    fn default_local_preset_has_empty_models() {
        let p = LocalPreset::default();
        assert!(p.stt_model.is_empty());
        assert!(p.translator_model.is_empty());
        assert!(p.suggester_model.is_empty());
        assert_eq!(p.ollama_endpoint, "http://localhost:11434");
    }

    // --- Ollama client (mock TCP server) ---

    use tokio::net::TcpListener;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn spawn_mock_ollama(body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    let mut buf = [0u8; 1024];
                    let _ = socket.read(&mut buf).await;
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
                        body.len(), body
                    );
                    let _ = socket.write_all(resp.as_bytes()).await;
                    let _ = socket.shutdown().await;
                });
            }
        });
        format!("http://{}", addr)
    }

    #[tokio::test]
    async fn ollama_list_models_parses_real_response() {
        let body = r#"{"models":[{"name":"llama3.2:3b","size":2000000000},{"name":"qwen2.5:7b","size":4700000000}]}"#;
        let url = spawn_mock_ollama(body).await;
        let models = OllamaClient::new(&url).list_models().await.unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].name, "llama3.2:3b");
    }

    #[tokio::test]
    async fn ollama_list_models_handles_connection_refused() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let url = format!("http://{}", addr);
        let res = OllamaClient::new(&url).list_models().await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn ollama_list_models_handles_invalid_json() {
        let body = "not json {";
        let url = spawn_mock_ollama(body).await;
        let res = OllamaClient::new(&url).list_models().await;
        assert!(res.is_err());
    }

    // --- Readiness checks (integration with mock Ollama) ---

    #[tokio::test]
    async fn readiness_ollama_not_running_when_endpoint_unreachable() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let mut cfg = RealtimeTranslateConfig::default();
        cfg.translator.api_url = format!("http://{}", addr);
        cfg.suggester.api_url = format!("http://{}", addr);

        let readiness = check_local_readiness(&cfg).await;
        assert!(matches!(readiness.ollama, ProviderStatus::NotRunning { .. }));
    }

    #[tokio::test]
    async fn readiness_ollama_model_missing_when_not_found() {
        let body = r#"{"models":[{"name":"llama3.2:3b","size":2000000000}]}"#;
        let url = spawn_mock_ollama(body).await;

        let mut cfg = RealtimeTranslateConfig::default();
        cfg.translator.api_url = url.clone();
        cfg.translator.extra = "qwen2.5:7b".into();
        cfg.suggester.api_url = url;

        let readiness = check_local_readiness(&cfg).await;
        assert!(matches!(readiness.ollama, ProviderStatus::ModelMissing { .. }));
    }

    #[tokio::test]
    async fn check_local_readiness_aggregator_produces_both() {
        let cfg = RealtimeTranslateConfig::default();
        let r = check_local_readiness(&cfg).await;
        assert!(matches!(r.ollama, ProviderStatus::NotRunning { .. }));
        assert!(matches!(r.whisper, ProviderStatus::NotInstalled { .. }));
    }
}
