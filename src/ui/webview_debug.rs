//! WebView2 debug helpers for RDP black screen diagnosis

/// Initialize WebView2 debug logging to file
/// Call this at app startup
pub fn init_debug_logging() {
    #[cfg(target_os = "windows")]
    {
        // Create logs directory
        let logs_dir = std::path::Path::new("target").join("debug_logs");
        let _ = std::fs::create_dir_all(&logs_dir);

        let log_file = logs_dir.join("webview2.log");
        log::info!("WebView2 debug log: {:?}", log_file);

        // Write initial debug info
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&log_file)
        {
            let _ = writeln!(file, "=== WebView2 Debug Log Started ===");
            let _ = writeln!(file, "App Version: {}", env!("CARGO_PKG_VERSION"));
            let _ = writeln!(file, "=== End Initial Info ===\n");
        }
    }
}

/// Log WebView2 environment details
pub fn log_webview2_info() {
    #[cfg(target_os = "windows")]
    {
        use wry::WebViewBuilderExtWindows;

        let builder = wry::WebViewBuilder::new();

        // Check WebView2 environment
        if let Ok(env) = builder.webview_environment() {
            log::info!("WebView2 environment created");
            log::info!("  Browser executable: {:?}", env.browser_executable_path());
            log::info!("  Browser version: {:?}", env.browser_version());
        } else {
            log::warn!("Failed to create WebView2 environment");
        }
    }
}
