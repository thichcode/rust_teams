//! Console management - hide console window after startup

/// Hide the console window after a delay
/// This allows seeing startup messages then auto-hiding
pub fn auto_hide_console(delay_ms: u64) {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = delay_ms;
    }

    #[cfg(target_os = "windows")]
    {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));

            unsafe {
                use winapi::um::wincon::GetConsoleWindow;
                use winapi::um::winuser::{SW_HIDE, ShowWindow};

                let console_window = GetConsoleWindow();
                if !console_window.is_null() {
                    ShowWindow(console_window, SW_HIDE);
                    log::info!("Console window hidden after {}ms", delay_ms);
                }
            }
        });
    }
}

/// Show the console window (for debugging)
#[allow(dead_code)]
pub fn show_console() {
    #[cfg(target_os = "windows")]
    {
        unsafe {
            use winapi::um::wincon::GetConsoleWindow;
            use winapi::um::winuser::{SW_SHOW, ShowWindow};

            let console_window = GetConsoleWindow();
            if !console_window.is_null() {
                ShowWindow(console_window, SW_SHOW);
            }
        }
    }
}
