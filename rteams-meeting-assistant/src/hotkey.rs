use std::sync::mpsc;
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::{HotKey, Modifiers, Code};
use global_hotkey::GlobalHotKeyEvent;

pub enum HotkeyEvent {
    ToggleRecording,
}

pub struct HotkeyManager {
    _manager: GlobalHotKeyManager,
    pub rx: mpsc::Receiver<HotkeyEvent>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let manager = GlobalHotKeyManager::new().expect("global hotkey manager");

        let toggle_key = HotKey::new(Some(Modifiers::CONTROL), Code::Space);
        manager.register(toggle_key).expect("register ctrl+space");

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