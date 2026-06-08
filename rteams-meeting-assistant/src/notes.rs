use std::path::PathBuf;
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
        r#"# Meeting Notes

**Date:** {}
**Duration:** In progress

---

## Transcript

{}
"#,
        Local::now().format("%Y-%m-%d %H:%M"),
        body,
    );

    std::fs::write(&path, content)?;
    log::info!("Saved transcript to {}", path.display());
    Ok(path)
}
