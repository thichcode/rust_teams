use std::sync::mpsc;

pub enum TrayCommand {
    ToggleVisibility,
    StopRecording,
    Quit,
}

pub struct TrayManager {
    _tray: tray_icon::TrayIcon,
    pub rx: mpsc::Receiver<TrayCommand>,
}

impl TrayManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        let show_hide = tray_icon::menu::MenuItem::new("Show/Hide", true, None);
        let stop_rec = tray_icon::menu::MenuItem::new("Stop Recording", true, None);
        let quit = tray_icon::menu::MenuItem::new("Quit", true, None);

        let show_hide_id = show_hide.id().clone();
        let stop_rec_id = stop_rec.id().clone();
        let quit_id = quit.id().clone();

        let menu = tray_icon::menu::Menu::with_items(&[&show_hide, &stop_rec, &quit]).unwrap();

        let icon = tray_icon::Icon::from_rgba(vec![0u8; 64 * 64 * 4], 64, 64).unwrap();

        let tray = tray_icon::TrayIconBuilder::new()
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .build()
            .unwrap();

        let event_rx = tray_icon::menu::MenuEvent::receiver();
        let tx_clone = tx.clone();
        std::thread::spawn(move || {
            while let Ok(event) = event_rx.recv() {
                let cmd = if event.id == show_hide_id {
                    TrayCommand::ToggleVisibility
                } else if event.id == stop_rec_id {
                    TrayCommand::StopRecording
                } else if event.id == quit_id {
                    TrayCommand::Quit
                } else {
                    continue;
                };
                let _ = tx_clone.send(cmd);
            }
        });

        Self { _tray: tray, rx }
    }
}
