//! Main application window management using winit
use anyhow::Result;
use serde::{Deserialize, Serialize};
use winit::{
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

pub struct WindowManager {
    window: Option<Window>,
    event_loop: Option<EventLoop<()>>,
}

impl WindowManager {
    pub fn new() -> Result<Self> {
        let event_loop = EventLoop::new()?;
        
        Ok(Self {
            window: None,
            event_loop: Some(event_loop),
        })
    }

    pub fn create_window(&mut self, settings: &WindowSettings) -> Result<()> {
        let event_loop = self.event_loop.take()
            .expect("Event loop already used");
        
        let mut builder = WindowBuilder::new()
            .with_title("Rust Teams")
            .with_inner_size(winit::dpi::LogicalSize::new(
                settings.width as f64,
                settings.height as f64,
            ))
            .with_maximized(settings.maximized)
            .with_visible(false);
        
        if let (Some(x), Some(y)) = (settings.x, settings.y) {
            builder = builder.with_position(winit::dpi::LogicalPosition::new(
                x as f64,
                y as f64,
            ));
        }
        
        let window = builder.build(&event_loop)?;
        
        self.window = Some(window);
        self.event_loop = Some(event_loop);
        
        Ok(())
    }

    pub fn show(&self) -> Result<()> {
        if let Some(window) = &self.window {
            window.set_visible(true);
        }
        Ok(())
    }

    pub fn hide(&self) -> Result<()> {
        if let Some(window) = &self.window {
            window.set_visible(false);
        }
        Ok(())
    }

    pub fn toggle(&self) -> Result<()> {
        if let Some(window) = &self.window {
            let visible = window.is_visible().unwrap_or(false);
            window.set_visible(!visible);
        }
        Ok(())
    }

    pub fn set_size(&self, width: u32, height: u32) -> Result<()> {
        if let Some(window) = &self.window {
            window.request_inner_size(winit::dpi::LogicalSize::new(
                width as f64,
                height as f64,
            ));
        }
        Ok(())
    }

    pub fn set_position(&self, x: i32, y: i32) -> Result<()> {
        if let Some(window) = &self.window {
            window.set_outer_position(winit::dpi::LogicalPosition::new(
                x as f64,
                y as f64,
            ));
        }
        Ok(())
    }

    pub fn maximize(&self) -> Result<()> {
        if let Some(window) = &self.window {
            window.set_maximized(true);
        }
        Ok(())
    }

    pub fn restore(&self) -> Result<()> {
        if let Some(window) = &self.window {
            window.set_maximized(false);
        }
        Ok(())
    }

    pub fn close(&self) -> Result<()> {
        if let Some(window) = &self.window {
            window.set_visible(false);
        }
        Ok(())
    }
    
    pub fn run_event_loop(&mut self) -> Result<()> {
        let event_loop = self.event_loop.take()
            .expect("Event loop not initialized");
        
        event_loop.run(|event, elwt| {
            match event {
                winit::event::Event::WindowEvent { event, window_id } => {
                    if let Some(window) = &self.window {
                        if window_id == window.id() {
                            match event {
                                winit::event::WindowEvent::CloseRequested => {
                                    elwt.exit();
                                }
                                winit::event::WindowEvent::Destroyed => {
                                    elwt.exit();
                                }
                                _ => {}
                            }
                        }
                    }
                }
                winit::event::Event::AboutToWait => {
                    elwt.set_control_flow(ControlFlow::Wait);
                }
                _ => {}
            }
        })?;
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSettings {
    pub width: u32,
    pub height: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub maximized: bool,
    pub always_on_top: bool,
    pub transparent: bool,
}
