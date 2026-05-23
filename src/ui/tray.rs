//! System tray icon - stub implementation
use anyhow::Result;

pub struct TrayIcon;

impl TrayIcon {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn set_icon(&self, _icon_type: &str) -> Result<()> {
        Ok(())
    }

    pub fn set_tooltip(&self, _tooltip: &str) -> Result<()> {
        Ok(())
    }

    pub fn update_menu(&self, _menu_items: &[String]) -> Result<()> {
        Ok(())
    }

    pub fn show(&self) -> Result<()> {
        Ok(())
    }

    pub fn hide(&self) -> Result<()> {
        Ok(())
    }

    pub fn set_visible(&self, _visible: bool) -> Result<()> {
        Ok(())
    }

    pub fn add_handler(&mut self, _handler: Box<dyn TrayEventHandler>) -> Result<()> {
        Ok(())
    }
}

pub trait TrayEventHandler {}
impl TrayEventHandler for () {}
