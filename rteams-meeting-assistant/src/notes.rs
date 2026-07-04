use std::path::PathBuf;
use std::sync::mpsc;
use anyhow::Result;
use chrono::{Local, Utc};

pub fn save_transcript(
    transcript: &[String],
    notes_dir: &str,
) -> Result<PathBuf> {
    let dir = PathBuf::from(notes_dir);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("meeting_{}.md", Utc::now().format("%Y%m%d_%H%M%S")));

    let body = transcript
        .iter()
        .enumerate()
        .map(|(i, l)| format!("{}. {}", i + 1, l))
        .collect::<Vec<_>>()
        .join("\n");

    let content = format!(
        "# Meeting Notes\n\n**Date:** {}\n**Duration:** In progress\n\n---\n\n## Transcript\n\n{}\n",
        Local::now().format("%Y-%m-%d %H:%M"),
        body,
    );

    std::fs::write(&path, content)?;
    log::info!("Saved transcript to {}", path.display());
    Ok(path)
}

pub fn list_notes(notes_dir: &str) -> Vec<PathBuf> {
    let dir = PathBuf::from(notes_dir);
    if !dir.exists() {
        return Vec::new();
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
        .map(|e| e.path())
        .collect();
    entries.sort_by(|a, b| {
        let ma = a.metadata().ok().and_then(|m| m.modified().ok());
        let mb = b.metadata().ok().and_then(|m| m.modified().ok());
        mb.cmp(&ma).then_with(|| a.cmp(b))
    });
    entries
}

pub fn spawn_summarize(
    transcript: Vec<String>,
    endpoint: &str,
    model: &str,
    tx: &mpsc::Sender<String>,
) {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            log::error!("Failed to create tokio runtime: {e}");
            return;
        }
    };
    let _ = rt.block_on(async {
        let body = serde_json::json!({
            "model": model,
            "prompt": format!(
                "Summarize this meeting transcript concisely. Highlight key points, decisions, and action items:\n\n{}",
                transcript.join("\n")
            ),
            "stream": false,
            "options": { "temperature": 0.3, "num_predict": 1024 }
        });
        let client = reqwest::Client::new();
        let url = format!("{}/api/generate", endpoint);
        match client.post(&url).json(&body).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let err = resp.text().await.unwrap_or_default();
                    log::error!("Ollama summary: {err}");
                    return;
                }
                if let Ok(v) = resp.json::<serde_json::Value>().await {
                    if let Some(text) = v["response"].as_str() {
                        let _ = tx.send(text.to_string());
                    }
                }
            }
            Err(e) => {
                log::error!("Ollama summary request failed: {e}");
            }
        }
    });
}
