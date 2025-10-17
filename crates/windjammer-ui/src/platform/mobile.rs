//! Mobile platform implementation (iOS/Android)

use super::capabilities::Capability;
use super::{Platform, PlatformType};

/// Mobile platform implementation
pub struct MobilePlatform {
    initialized: bool,
}

impl MobilePlatform {
    /// Create a new mobile platform instance
    pub fn new() -> Self {
        Self { initialized: false }
    }

    /// Detect if running on iOS
    pub fn is_ios() -> bool {
        cfg!(target_os = "ios")
    }

    /// Detect if running on Android
    pub fn is_android() -> bool {
        cfg!(target_os = "android")
    }
}

impl Default for MobilePlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for MobilePlatform {
    fn platform_type(&self) -> PlatformType {
        PlatformType::Mobile
    }

    fn init(&mut self) -> Result<(), String> {
        if self.initialized {
            return Ok(());
        }

        #[cfg(any(feature = "mobile-ios", feature = "mobile-android"))]
        {
            // Initialize mobile platform
            // Platform-specific initialization
        }

        self.initialized = true;
        Ok(())
    }

    fn run(&mut self) -> Result<(), String> {
        #[cfg(any(feature = "mobile-ios", feature = "mobile-android"))]
        {
            // Mobile event loop runs in the native runtime
            Ok(())
        }

        #[cfg(not(any(feature = "mobile-ios", feature = "mobile-android")))]
        {
            Err("Mobile platform not enabled".to_string())
        }
    }

    fn supports_capability(&self, capability: Capability) -> bool {
        match capability {
            // Mobile supports full device access
            Capability::Camera => true,
            Capability::Location => true,
            Capability::Contacts => true,
            Capability::Notifications => true,
            Capability::Biometrics => true,
            Capability::Haptics => true,
            Capability::FilePicker => true,
            Capability::Clipboard => true,
            Capability::WebSocket => true,
            Capability::LocalStorage => true,
            Capability::BackgroundTasks => true,

            // Limited filesystem access (sandboxed)
            Capability::Filesystem => cfg!(any(feature = "mobile-ios", feature = "mobile-android")),

            // Not supported on mobile
            Capability::SystemTray => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mobile_platform_creation() {
        let platform = MobilePlatform::new();
        assert_eq!(platform.platform_type(), PlatformType::Mobile);
        assert!(!platform.initialized);
    }

    #[test]
    fn test_mobile_platform_init() {
        let mut platform = MobilePlatform::new();
        assert!(platform.init().is_ok());
        assert!(platform.initialized);
    }

    #[test]
    fn test_mobile_capabilities() {
        let platform = MobilePlatform::new();

        // Mobile should support these
        assert!(platform.supports_capability(Capability::Camera));
        assert!(platform.supports_capability(Capability::Location));
        assert!(platform.supports_capability(Capability::Contacts));
        assert!(platform.supports_capability(Capability::Biometrics));
        assert!(platform.supports_capability(Capability::Haptics));

        // Mobile should NOT support these
        assert!(!platform.supports_capability(Capability::SystemTray));
    }
}
