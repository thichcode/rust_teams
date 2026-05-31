//! Taskbar badge and notification sound for Windows

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
        use winapi::um::winuser::{MessageBeep, MB_ICONASTERISK};
        MessageBeep(MB_ICONASTERISK);
    }
}

/// Update taskbar badge with unread count using Windows API
pub fn update_taskbar_badge(hwnd: isize, count: u32) {
    #[cfg(target_os = "windows")]
    unsafe {
        use winapi::um::winuser::{SetWindowTextW, FlashWindow};
        
        // Update window title with count at the beginning
        let title = if count > 0 {
            format!("({}) R Teams", count)
        } else {
            "R Teams".to_string()
        };
        
        // Convert to wide string
        let wide_title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        
        SetWindowTextW(hwnd as *mut _, wide_title.as_ptr());
        
        // Flash taskbar if there are new messages
        if count > 0 {
            FlashWindow(hwnd as *mut _, 1); // 1 = flash until foreground
        }
        
        log::info!("Badge updated: {} unread messages", count);
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
