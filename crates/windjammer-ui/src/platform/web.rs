//! Web platform implementation (JavaScript/WASM)

use super::capabilities::{Capability, CapabilityError, CapabilityResult};
use super::{Platform, PlatformType};

/// Web platform implementation
pub struct WebPlatform {
    initialized: bool,
}

impl WebPlatform {
    /// Create a new web platform instance
    pub fn new() -> Self {
        Self { initialized: false }
    }

    /// Check if running in a browser
    #[cfg(target_family = "wasm")]
    pub fn is_browser() -> bool {
        web_sys::window().is_some()
    }

    #[cfg(not(target_family = "wasm"))]
    pub fn is_browser() -> bool {
        false
    }
}

impl Default for WebPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for WebPlatform {
    fn platform_type(&self) -> PlatformType {
        PlatformType::Web
    }

    fn init(&mut self) -> Result<(), String> {
        if self.initialized {
            return Ok(());
        }

        #[cfg(target_family = "wasm")]
        {
            // Initialize web platform
            // Set panic hook for better error messages
            #[cfg(feature = "console_error_panic_hook")]
            console_error_panic_hook::set_once();
        }

        self.initialized = true;
        Ok(())
    }

    fn run(&mut self) -> Result<(), String> {
        // Web platform runs in the browser event loop
        // No custom event loop needed
        Ok(())
    }

    fn supports_capability(&self, capability: Capability) -> bool {
        match capability {
            // Web supports these via Web APIs
            Capability::WebSocket => true,
            Capability::LocalStorage => true,
            Capability::Clipboard => true,
            Capability::Notifications => true,

            // Limited support
            Capability::Camera => cfg!(target_family = "wasm"),
            Capability::Location => cfg!(target_family = "wasm"),
            Capability::FilePicker => true,
            Capability::Haptics => cfg!(target_family = "wasm"),

            // Not supported on web
            Capability::Filesystem => false,
            Capability::Contacts => false,
            Capability::SystemTray => false,
            Capability::Biometrics => false,
            Capability::BackgroundTasks => false,
        }
    }
}

/// Web-specific file picker API
pub struct FilePicker;

impl FilePicker {
    /// Pick a single file
    pub async fn pick_file() -> CapabilityResult<Vec<u8>> {
        #[cfg(target_family = "wasm")]
        {
            // Implementation would use <input type="file">
            // For now, return error
            Err(CapabilityError::Error("Not yet implemented".to_string()))
        }

        #[cfg(not(target_family = "wasm"))]
        {
            Err(CapabilityError::NotSupported(Capability::FilePicker))
        }
    }

    /// Pick multiple files
    pub async fn pick_files() -> CapabilityResult<Vec<Vec<u8>>> {
        #[cfg(target_family = "wasm")]
        {
            // Implementation would use <input type="file" multiple>
            Err(CapabilityError::Error("Not yet implemented".to_string()))
        }

        #[cfg(not(target_family = "wasm"))]
        {
            Err(CapabilityError::NotSupported(Capability::FilePicker))
        }
    }
}

/// Web-specific geolocation API
pub struct Geolocation;

#[derive(Debug, Clone)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: f64,
}

impl Geolocation {
    /// Get current location
    pub async fn current_location() -> CapabilityResult<Location> {
        #[cfg(target_family = "wasm")]
        {
            // Implementation would use navigator.geolocation
            Err(CapabilityError::Error("Not yet implemented".to_string()))
        }

        #[cfg(not(target_family = "wasm"))]
        {
            Err(CapabilityError::NotSupported(Capability::Location))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_platform_creation() {
        let platform = WebPlatform::new();
        assert_eq!(platform.platform_type(), PlatformType::Web);
        assert!(!platform.initialized);
    }

    #[test]
    fn test_web_platform_init() {
        let mut platform = WebPlatform::new();
        assert!(platform.init().is_ok());
        assert!(platform.initialized);

        // Second init should succeed
        assert!(platform.init().is_ok());
    }

    #[test]
    fn test_web_capabilities() {
        let platform = WebPlatform::new();

        // Web should support these
        assert!(platform.supports_capability(Capability::WebSocket));
        assert!(platform.supports_capability(Capability::LocalStorage));
        assert!(platform.supports_capability(Capability::Clipboard));

        // Web should NOT support these
        assert!(!platform.supports_capability(Capability::Filesystem));
        assert!(!platform.supports_capability(Capability::SystemTray));
        assert!(!platform.supports_capability(Capability::Contacts));
    }
}
