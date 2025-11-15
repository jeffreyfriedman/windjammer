//! Platform abstraction layer for cross-platform compatibility
//!
//! This module provides a unified API for different platforms:
//! - Web (JavaScript/WASM)
//! - Desktop (macOS, Windows, Linux via Tauri)
//! - Mobile (iOS, Android)

pub mod capabilities;
pub mod capability_impl;
// Desktop platform (old winit+wgpu implementation, deprecated)
// #[cfg(all(not(target_arch = "wasm32"), feature = "desktop-old"))]
// pub mod desktop;
// pub mod desktop_renderer;
pub mod mobile;
pub mod web;

use std::fmt;

/// The type of platform the application is running on
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformType {
    /// Web platform (browser, JavaScript or WASM)
    Web,
    /// Desktop platform (macOS, Windows, Linux)
    Desktop,
    /// Mobile platform (iOS, Android)
    Mobile,
}

impl fmt::Display for PlatformType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlatformType::Web => write!(f, "web"),
            PlatformType::Desktop => write!(f, "desktop"),
            PlatformType::Mobile => write!(f, "mobile"),
        }
    }
}

/// Trait for platform-specific implementations
pub trait Platform: Send + Sync {
    /// Get the platform type
    fn platform_type(&self) -> PlatformType;

    /// Initialize the platform
    fn init(&mut self) -> Result<(), String>;

    /// Run the event loop (if applicable)
    fn run(&mut self) -> Result<(), String>;

    /// Check if a capability is supported
    fn supports_capability(&self, capability: capabilities::Capability) -> bool;
}

/// Detect the current platform at compile time
pub fn detect_platform() -> PlatformType {
    #[cfg(all(
        target_family = "wasm",
        not(feature = "desktop"),
        not(feature = "mobile-ios"),
        not(feature = "mobile-android")
    ))]
    {
        return PlatformType::Web;
    }

    #[cfg(all(feature = "desktop", not(target_family = "wasm")))]
    {
        return PlatformType::Desktop;
    }

    #[cfg(any(feature = "mobile-ios", feature = "mobile-android"))]
    #[allow(unreachable_code)]
    {
        return PlatformType::Mobile;
    }

    #[cfg(not(any(
        target_family = "wasm",
        feature = "desktop",
        feature = "mobile-ios",
        feature = "mobile-android"
    )))]
    {
        // Default to web for unknown platforms
        return PlatformType::Web;
    }

    #[allow(unreachable_code)]
    PlatformType::Web
}

/// Create a platform instance for the current platform
pub fn create_platform() -> Box<dyn Platform> {
    match detect_platform() {
        PlatformType::Web => Box::new(web::WebPlatform::new()),
        PlatformType::Desktop => {
            // Desktop platform is now handled by eframe, not a separate platform module
            panic!("Desktop platform should use eframe directly, not create_platform()")
        }
        PlatformType::Mobile => Box::new(mobile::MobilePlatform::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform_type = detect_platform();
        assert!(matches!(
            platform_type,
            PlatformType::Web | PlatformType::Desktop | PlatformType::Mobile
        ));
    }

    #[test]
    fn test_platform_display() {
        assert_eq!(PlatformType::Web.to_string(), "web");
        assert_eq!(PlatformType::Desktop.to_string(), "desktop");
        assert_eq!(PlatformType::Mobile.to_string(), "mobile");
    }

    #[test]
    fn test_create_platform() {
        let platform = create_platform();
        assert_eq!(platform.platform_type(), detect_platform());
    }
}
