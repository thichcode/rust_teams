//! Taskbar badge and notification sound for Windows

#[cfg(target_os = "windows")]
use winapi::um::winuser::{MessageBeep, MB_ICONASTERISK};

use regex::Regex;

/// Parse unread count from Teams page title
/// Expected formats: "R Teams (3)", "R Teams - Chat (5)", etc.
pub fn parse_unread_count(title: &str) -> Option<u32> {
    let re = Regex::new(r"\((\d+)\)").ok()?;
    re.captures(title)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

/// Play system notification sound
pub fn play_notification_sound() {
    #[cfg(target_os = "windows")]
    unsafe {
        MessageBeep(MB_ICONASTERISK);
    }
}

/// Update taskbar badge with unread count
/// Note: Windows 10+ badge API requires COM initialization
/// For now, we log the count and play sound
#[allow(dead_code)]
pub fn update_taskbar_badge(_hwnd: isize, count: u32) {
    if count > 0 {
        log::info!("Badge update: {} unread messages", count);
    } else {
        log::info!("Badge cleared: no unread messages");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_unread_count() {
        assert_eq!(parse_unread_count("R Teams (3)"), Some(3));
        assert_eq!(parse_unread_count("R Teams - Chat (5)"), Some(5));
        assert_eq!(parse_unread_count("R Teams"), None);
        assert_eq!(parse_unread_count("R Teams (0)"), Some(0));
    }
}
