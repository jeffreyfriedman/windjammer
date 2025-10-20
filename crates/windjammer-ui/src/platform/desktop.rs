//! Desktop platform implementation (winit + wgpu)

use super::capabilities::Capability;
use super::{Platform, PlatformType};

#[cfg(feature = "desktop")]
use winit::window::Window;

/// Desktop platform implementation
pub struct DesktopPlatform {
    initialized: bool,
    #[cfg(feature = "desktop")]
    window: Option<std::sync::Arc<Window>>,
}

impl DesktopPlatform {
    /// Create a new desktop platform instance
    pub fn new() -> Self {
        Self {
            initialized: false,
            #[cfg(feature = "desktop")]
            window: None,
        }
    }

    /// Create a window with the given title and size
    #[cfg(feature = "desktop")]
    pub fn create_window(&mut self, title: &str, width: u32, height: u32) -> Result<(), String> {
        use winit::event_loop::EventLoop;
        use winit::window::WindowAttributes;

        let event_loop =
            EventLoop::new().map_err(|e| format!("Failed to create event loop: {}", e))?;

        let window_attrs = WindowAttributes::default()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(width, height));

        let window = event_loop
            .create_window(window_attrs)
            .map_err(|e| format!("Failed to create window: {}", e))?;

        self.window = Some(std::sync::Arc::new(window));
        Ok(())
    }

    /// Get a reference to the window
    #[cfg(feature = "desktop")]
    pub fn window(&self) -> Option<std::sync::Arc<Window>> {
        self.window.clone()
    }
}

impl Default for DesktopPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for DesktopPlatform {
    fn platform_type(&self) -> PlatformType {
        PlatformType::Desktop
    }

    fn init(&mut self) -> Result<(), String> {
        if self.initialized {
            return Ok(());
        }

        #[cfg(feature = "desktop")]
        {
            // Create default window if not already created
            if self.window.is_none() {
                self.create_window("Windjammer App", 800, 600)?;
            }
        }

        self.initialized = true;
        Ok(())
    }

    fn run(&mut self) -> Result<(), String> {
        if !self.initialized {
            self.init()?;
        }

        #[cfg(feature = "desktop")]
        {
            // Event loop must be created and run in the same scope
            // This is a limitation of winit's design
            use winit::event::{Event, WindowEvent};
            use winit::event_loop::{ControlFlow, EventLoop};

            let event_loop =
                EventLoop::new().map_err(|e| format!("Failed to create event loop: {}", e))?;

            event_loop
                .run(move |event, elwt| {
                    elwt.set_control_flow(ControlFlow::Wait);

                    match event {
                        Event::WindowEvent {
                            event: WindowEvent::CloseRequested,
                            ..
                        } => {
                            elwt.exit();
                        }
                        Event::WindowEvent {
                            event: WindowEvent::RedrawRequested,
                            ..
                        } => {
                            // Render frame
                            // TODO: Integrate with wgpu renderer
                        }
                        _ => {}
                    }
                })
                .map_err(|e| format!("Event loop error: {}", e))?;

            Ok(())
        }

        #[cfg(not(feature = "desktop"))]
        {
            Err("Desktop platform not enabled".to_string())
        }
    }

    fn supports_capability(&self, capability: Capability) -> bool {
        match capability {
            // Desktop supports full OS access
            Capability::Filesystem => true,
            Capability::FilePicker => true,
            Capability::SystemTray => true,
            Capability::Clipboard => true,
            Capability::Notifications => true,
            Capability::WebSocket => true,
            Capability::LocalStorage => true,
            Capability::BackgroundTasks => true,

            // Limited support
            Capability::Camera => cfg!(feature = "desktop"),
            Capability::Location => cfg!(feature = "desktop"),

            // Not typically supported on desktop
            Capability::Contacts => false,
            Capability::Biometrics => false,
            Capability::Haptics => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_platform_creation() {
        let platform = DesktopPlatform::new();
        assert_eq!(platform.platform_type(), PlatformType::Desktop);
        assert!(!platform.initialized);
    }

    #[test]
    fn test_desktop_platform_init() {
        let mut platform = DesktopPlatform::new();
        assert!(platform.init().is_ok());
        assert!(platform.initialized);
    }

    #[test]
    fn test_desktop_capabilities() {
        let platform = DesktopPlatform::new();

        // Desktop should support these
        assert!(platform.supports_capability(Capability::Filesystem));
        assert!(platform.supports_capability(Capability::SystemTray));
        assert!(platform.supports_capability(Capability::FilePicker));
        assert!(platform.supports_capability(Capability::Notifications));

        // Desktop should NOT support these
        assert!(!platform.supports_capability(Capability::Contacts));
        assert!(!platform.supports_capability(Capability::Haptics));
    }
}
