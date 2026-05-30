//! System tray icon implementation for R Teams
use anyhow::Result;
use tao::event_loop::EventLoopWindowTarget;
use tao::system_tray::{SystemTray, SystemTrayBuilder};
use tao::menu::{MenuItem, MenuBuilder};
use tao::window::Icon;

pub struct TrayIcon {
    system_tray: Option<SystemTray<()>>,
}

impl TrayIcon {
    pub fn new() -> Result<Self> {
        Ok(Self {
            system_tray: None,
        })
    }

    /// Initialize the system tray with icon and menu
    pub fn initialize(&mut self, event_loop: &EventLoopWindowTarget<()>) -> Result<()> {
        // Create context menu
        let menu = MenuBuilder::new()
            .add_item(MenuItem::new("Show R Teams"))
            .add_native_item(MenuItem::Separator)
            .add_item(MenuItem::new("Quit"))
            .build();

        // Load tray icon
        let icon = load_tray_icon().unwrap_or_else(|e| {
            log::warn!("Failed to load tray icon: {}, using default", e);
            create_default_icon()
        });

        // Build system tray
        let system_tray = SystemTrayBuilder::new(icon, menu)
            .build(event_loop)
            .map_err(|e| format!("Failed to create system tray: {}", e))?;

        self.system_tray = Some(system_tray);
        log::info!("System tray initialized");
        Ok(())
    }

    pub fn set_tooltip(&self, _tooltip: &str) -> Result<()> {
        // TODO: Implement tooltip update
        Ok(())
    }

    pub fn update_menu(&self, _menu_items: &[String]) -> Result<()> {
        // TODO: Implement menu update
        Ok(())
    }

    pub fn show(&self) -> Result<()> {
        // TODO: Implement show
        Ok(())
    }

    pub fn hide(&self) -> Result<()> {
        // TODO: Implement hide
        Ok(())
    }

    pub fn set_visible(&self, _visible: bool) -> Result<()> {
        // TODO: Implement visibility toggle
        Ok(())
    }
}

/// Load tray icon from embedded RGBA data
fn load_tray_icon() -> Result<Icon> {
    // 16x16 Teams purple icon with white "R"
    let size: u32 = 16;
    let mut rgba = vec![0u8; (size * size * 4) as usize];

    for y in 0..size {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;

            // Simple "R" letter approximation
            let is_r = (x >= 4 && x <= 12 && y >= 3 && y <= 13)
                && ((x <= 6)
                    || (y <= 5)
                    || (y >= 9 && x >= 6 && (x + y) <= 16));

            if is_r {
                // White for "R"
                rgba[idx] = 255;
                rgba[idx + 1] = 255;
                rgba[idx + 2] = 255;
                rgba[idx + 3] = 255;
            } else {
                // Teams purple background
                rgba[idx] = 98;  // R
                rgba[idx + 1] = 100; // G
                rgba[idx + 2] = 167; // B
                rgba[idx + 3] = 255; // A
            }
        }
    }

    let icon = Icon::from_rgba(rgba, size, size)
        .map_err(|e| format!("Failed to create tray icon: {}", e))?;

    Ok(icon)
}

/// Create a default 16x16 icon (Teams purple with "R")
fn create_default_icon() -> Icon {
    load_tray_icon().unwrap_or_else(|_| {
        // Fallback: solid purple square
        let mut rgba = vec![0u8; (16 * 16 * 4) as usize];
        for chunk in rgba.chunks_mut(4) {
            chunk[0] = 98;
            chunk[1] = 100;
            chunk[2] = 167;
            chunk[3] = 255;
        }
        Icon::from_rgba(rgba, 16, 16).expect("Failed to create fallback icon")
    })
}

pub trait TrayEventHandler {}
impl TrayEventHandler for () {}
