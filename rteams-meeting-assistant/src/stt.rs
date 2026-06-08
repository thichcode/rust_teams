use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait SttProvider: Send + Sync {
    async fn transcribe(&self, audio: &[f32], language: &str) -> Result<String>;
}

pub struct LocalWhisper {
    whisper_bin: String,
    model_path: String,
}

impl LocalWhisper {
    pub fn new(whisper_bin: &str, model_path: &str) -> Self {
        Self {
            whisper_bin: whisper_bin.to_string(),
            model_path: model_path.to_string(),
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
