use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};

pub fn export_txt(history: &[String], dir: &Path) -> std::io::Result<PathBuf> {
    let ts = Local::now().format("%Y%m%d-%H%M%S");
    let path = dir.join(format!("transcript-{ts}.txt"));
    let content = history.join("\n");
    fs::write(&path, &content)?;
    Ok(path)
}

pub fn export_md(history: &[String], dir: &Path) -> std::io::Result<PathBuf> {
    let now = Local::now();
    let ts = now.format("%Y-%m-%d %H:%M:%S");
    let filename = now.format("%Y%m%d-%H%M%S");
    let path = dir.join(format!("transcript-{filename}.md"));
    let mut content = format!("# Transcript — {ts}\n\n");
    for line in history {
        if let Some(speaker_end) = line.find(']') {
            let label = &line[..speaker_end + 1];
            let text = &line[speaker_end + 1..];
            content.push_str(&format!("### {label}\n{text}\n\n"));
        } else {
            content.push_str(&format!("{line}\n\n"));
        }
    }
    fs::write(&path, &content)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_txt() {
        let dir = std::env::temp_dir().join("rteams-test-export");
        let _ = fs::create_dir_all(&dir);
        let history = vec![
            "[Speaker 1] Hello world".to_string(),
            "[Speaker 2] Hi there".to_string(),
        ];
        let path = export_txt(&history, &dir).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("Hello world"));
        assert!(content.contains("Hi there"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_export_md() {
        let dir = std::env::temp_dir().join("rteams-test-export-md");
        let _ = fs::create_dir_all(&dir);
        let history = vec![
            "[Speaker 1] Hello world".to_string(),
            "[Speaker 2] Hi there".to_string(),
        ];
        let path = export_md(&history, &dir).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("### [Speaker 1]"));
        assert!(content.contains("Hello world"));
        assert!(content.contains("### [Speaker 2]"));
        assert!(content.contains("Hi there"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_export_empty() {
        let dir = std::env::temp_dir().join("rteams-test-export-empty");
        let _ = fs::create_dir_all(&dir);
        let path = export_txt(&[], &dir).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "");
        let _ = fs::remove_dir_all(&dir);
    }
}
