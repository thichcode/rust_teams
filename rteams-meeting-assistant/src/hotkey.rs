use global_hotkey::GlobalHotKeyEvent;
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use std::sync::mpsc;

pub enum HotkeyEvent {
    ToggleRecording,
}

pub struct HotkeyManager {
    _manager: GlobalHotKeyManager,
    pub rx: mpsc::Receiver<HotkeyEvent>,
}

/// Parse a hotkey string like "Ctrl+Space", "Alt+R", "Ctrl+Shift+M".
/// Returns None for invalid or empty strings (hotkey disabled).
fn parse_hotkey(hotkey_str: &str) -> Option<HotKey> {
    let trimmed = hotkey_str.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parts: Vec<&str> = trimmed.split('+').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return None;
    }

    let mut modifiers = Modifiers::empty();
    let key_part = parts[parts.len() - 1];

    for mod_part in &parts[..parts.len() - 1] {
        match *mod_part {
            "Ctrl" | "CONTROL" | "ctrl" => modifiers |= Modifiers::CONTROL,
            "Alt" | "ALT" | "alt" => modifiers |= Modifiers::ALT,
            "Shift" | "SHIFT" | "shift" => modifiers |= Modifiers::SHIFT,
            "Win" | "WIN" | "win" | "Meta" | "META" | "meta" | "Super" | "SUPER" | "super" => {
                modifiers |= Modifiers::SUPER;
            }
            _ => {
                log::warn!("Unknown modifier in hotkey: {mod_part}");
                return None;
            }
        }
    }

    let code = match key_part {
        "Space" | "space" | "SPACE" => Code::Space,
        "Enter" | "enter" | "ENTER" | "Return" | "return" => Code::Enter,
        "Tab" | "tab" => Code::Tab,
        "Escape" | "escape" | "Esc" | "esc" => Code::Escape,
        "Backspace" | "backspace" => Code::Backspace,
        "Delete" | "delete" | "Del" | "del" => Code::Delete,
        "Home" | "home" => Code::Home,
        "End" | "end" => Code::End,
        "PageUp" | "pageup" | "PgUp" => Code::PageUp,
        "PageDown" | "pagedown" | "PgDn" => Code::PageDown,
        "Up" | "up" | "ArrowUp" => Code::ArrowUp,
        "Down" | "down" | "ArrowDown" => Code::ArrowDown,
        "Left" | "left" | "ArrowLeft" => Code::ArrowLeft,
        "Right" | "right" | "ArrowRight" => Code::ArrowRight,
        // Single letters A-Z
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,
        // Number row
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        // Function keys
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,
        _ => {
            log::warn!("Unknown key in hotkey: {key_part}");
            return None;
        }
    };

    Some(HotKey::new(Some(modifiers), code))
}

impl HotkeyManager {
    pub fn new(hotkey_str: &str) -> Self {
        let (tx, rx) = mpsc::channel();
        let manager = GlobalHotKeyManager::new().expect("global hotkey manager");

        if let Some(key) = parse_hotkey(hotkey_str) {
            match manager.register(key) {
                Ok(_) => log::info!("Global hotkey registered: {hotkey_str}"),
                Err(e) => log::warn!("Failed to register hotkey {hotkey_str}: {e}"),
            }
        } else {
            log::info!("No global hotkey configured (toggle_hotkey is empty or invalid)");
        }

        let tx_clone = tx.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state == global_hotkey::HotKeyState::Pressed {
                let _ = tx_clone.send(HotkeyEvent::ToggleRecording);
            }
        }));

        Self {
            _manager: manager,
            rx,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ctrl_space() {
        let hk = parse_hotkey("Ctrl+Space");
        assert!(hk.is_some());
    }

    #[test]
    fn test_parse_alt_r() {
        let hk = parse_hotkey("Alt+R");
        assert!(hk.is_some());
    }

    #[test]
    fn test_parse_ctrl_shift_m() {
        let hk = parse_hotkey("Ctrl+Shift+M");
        assert!(hk.is_some());
    }

    #[test]
    fn test_parse_empty() {
        assert!(parse_hotkey("").is_none());
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_hotkey("Invalid+Key+Combo").is_none());
    }
}
