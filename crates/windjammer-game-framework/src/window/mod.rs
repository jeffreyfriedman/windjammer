//! Window management with winit integration
//!
//! Design Philosophy:
//! - Simple window creation
//! - Event loop integration
//! - Cross-platform (desktop, eventually mobile)
//! - Integrates with wgpu for rendering

use winit::{
    event::Event,
    event_loop::EventLoop,
    window::{Window as WinitWindow, WindowBuilder},
};

/// Window configuration
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub vsync: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Windjammer Game".to_string(),
            width: 800,
            height: 600,
            resizable: true,
            vsync: true,
        }
    }
}

/// Window wrapper
pub struct Window {
    window: WinitWindow,
    event_loop: Option<EventLoop<()>>,
}

impl Window {
    /// Create a new window
    pub fn new(config: WindowConfig) -> Result<Self, String> {
        let event_loop =
            EventLoop::new().map_err(|e| format!("Failed to create event loop: {}", e))?;

        let window = WindowBuilder::new()
            .with_title(config.title)
            .with_inner_size(winit::dpi::PhysicalSize::new(config.width, config.height))
            .with_resizable(config.resizable)
            .build(&event_loop)
            .map_err(|e| format!("Failed to create window: {}", e))?;

        Ok(Self {
            window,
            event_loop: Some(event_loop),
        })
    }

    /// Get the inner window
    pub fn inner(&self) -> &WinitWindow {
        &self.window
    }

    /// Get window size
    pub fn size(&self) -> (u32, u32) {
        let size = self.window.inner_size();
        (size.width, size.height)
    }

    /// Run the event loop with a game loop callback
    pub fn run<F>(mut self, mut callback: F) -> Result<(), String>
    where
        F: FnMut(&WinitWindow, &Event<()>, &winit::event_loop::EventLoopWindowTarget<()>) + 'static,
    {
        let event_loop = self
            .event_loop
            .take()
            .ok_or("Event loop already consumed")?;

        event_loop
            .run(move |event, elwt| {
                callback(&self.window, &event, elwt);
            })
            .map_err(|e| format!("Event loop error: {}", e))
    }
}

/// Simple window runner for games
///
/// Note: Full implementation requires winit 0.29+ API integration
/// This is a placeholder for the architecture
pub struct WindowRunner {
    _config: WindowConfig,
}

impl WindowRunner {
    /// Create a new window runner
    pub fn new(config: WindowConfig) -> Result<Self, String> {
        Ok(Self { _config: config })
    }

    /// Run the game with the window
    ///
    /// TODO: Complete implementation with winit event loop
    pub fn run<G, U, R>(self, _game: G, _update: U, _render: R) -> Result<(), String>
    where
        G: 'static,
        U: FnMut(&mut G, f32) + 'static,
        R: FnMut(&mut G) + 'static,
    {
        // Placeholder - full implementation requires proper winit integration
        Err("WindowRunner not yet fully implemented - see window/mod.rs".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_config_default() {
        let config = WindowConfig::default();
        assert_eq!(config.title, "Windjammer Game");
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
        assert!(config.resizable);
        assert!(config.vsync);
    }

    #[test]
    fn test_window_config_custom() {
        let config = WindowConfig {
            title: "My Game".to_string(),
            width: 1920,
            height: 1080,
            resizable: false,
            vsync: false,
        };
        assert_eq!(config.title, "My Game");
        assert_eq!(config.width, 1920);
    }
}
