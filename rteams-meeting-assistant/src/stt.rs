use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;

#[async_trait]
pub trait SttProvider: Send + Sync {
    async fn transcribe(&self, audio: &[f32], language: &str) -> Result<String>;
}

/// Whisper.cpp subprocess-based STT.
///
/// **Known limitation:** each `transcribe()` call spawns a new `whisper.cpp`
/// process which loads the model from disk (~1-3s overhead per utterance).
/// For production use, prefer:
///   a) `whisper-rs` crate — in-process inference via C++ bindings
///   b) whisper.cpp server mode (./server —listen) — keep process warm
///   c) Keep the subprocess alive via stdin/stdout pipe
pub struct LocalWhisper {
    whisper_bin: String,
    model_path: String,
    /// Optional persistent child process handle (kept warm across calls).
    #[allow(dead_code)]
    child: Option<std::process::Child>,
}

impl LocalWhisper {
    pub fn new(whisper_bin: &str, model_path: &str) -> Self {
        Self {
            whisper_bin: whisper_bin.to_string(),
            model_path: model_path.to_string(),
            child: None,
        }
    }
}

#[async_trait]
impl SttProvider for LocalWhisper {
    async fn transcribe(&self, audio: &[f32], language: &str) -> Result<String> {
        let wav = crate::audio::AudioCapture::to_wav(audio, 16000, 1)?;
        let tmp_dir = std::env::temp_dir();
        let id = uuid::Uuid::new_v4();
        let tmp_wav = tmp_dir.join(format!("whisper_{id}.wav"));
        let tmp_out = tmp_wav.with_extension("txt");
        std::fs::write(&tmp_wav, &wav)?;

        let lang = if language.is_empty() { "auto" } else { language };
        let output = tokio::process::Command::new(&self.whisper_bin)
            .arg("-m").arg(&self.model_path)
            .arg("-f").arg(tmp_wav.as_os_str())
            .arg("-otxt").arg("-l").arg(lang)
            .arg("--no-prints")
            .stdin(Stdio::null())
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("whisper: {e}"))?;

        let _ = std::fs::remove_file(&tmp_wav);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("whisper exit {}: {stderr}", output.status);
        }

        let text = tokio::fs::read_to_string(&tmp_out)
            .await
            .unwrap_or_else(|_| String::from_utf8_lossy(&output.stdout).to_string());
        let _ = std::fs::remove_file(&tmp_out);

        Ok(text.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_fields() {
        let stt = LocalWhisper::new("/usr/bin/whisper", "/models/tiny.bin");
        assert_eq!(stt.whisper_bin, "/usr/bin/whisper");
        assert_eq!(stt.model_path, "/models/tiny.bin");
    }

    #[test]
    fn test_trait_object() {
        let stt = LocalWhisper::new("whisper", "model.bin");
        let provider: &dyn SttProvider = &stt;
        // Trait is object-safe for the async fn because we use #[async_trait]
        assert!(std::mem::size_of_val(provider) > 0);
    }
}
