//! Desktop platform implementation (Tauri)

use super::capabilities::Capability;
use super::{Platform, PlatformType};

/// Desktop platform implementation
pub struct DesktopPlatform {
    initialized: bool,
}

impl DesktopPlatform {
    /// Create a new desktop platform instance
    pub fn new() -> Self {
        Self { initialized: false }
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
            // Initialize Tauri
            // This would be done by the Tauri runtime
        }

        self.initialized = true;
        Ok(())
    }

    fn run(&mut self) -> Result<(), String> {
        #[cfg(feature = "desktop")]
        {
            // Tauri event loop
            // This would be handled by Tauri's runtime
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
