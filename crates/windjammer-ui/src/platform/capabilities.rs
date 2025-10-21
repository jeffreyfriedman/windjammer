//! Platform capabilities abstraction
//!
//! Defines capabilities that may or may not be available on different platforms

use std::fmt;

/// Platform capabilities that can be queried
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Filesystem access
    Filesystem,
    /// Camera access
    Camera,
    /// GPS/location services
    Location,
    /// Contacts access
    Contacts,
    /// Push notifications
    Notifications,
    /// System tray (desktop)
    SystemTray,
    /// File picker dialogs
    FilePicker,
    /// Clipboard access
    Clipboard,
    /// WebSocket support
    WebSocket,
    /// Local storage/persistent data
    LocalStorage,
    /// Biometric authentication (fingerprint, Face ID)
    Biometrics,
    /// Vibration/haptics
    Haptics,
    /// Background tasks
    BackgroundTasks,
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Capability::Filesystem => write!(f, "filesystem"),
            Capability::Camera => write!(f, "camera"),
            Capability::Location => write!(f, "location"),
            Capability::Contacts => write!(f, "contacts"),
            Capability::Notifications => write!(f, "notifications"),
            Capability::SystemTray => write!(f, "system_tray"),
            Capability::FilePicker => write!(f, "file_picker"),
            Capability::Clipboard => write!(f, "clipboard"),
            Capability::WebSocket => write!(f, "websocket"),
            Capability::LocalStorage => write!(f, "local_storage"),
            Capability::Biometrics => write!(f, "biometrics"),
            Capability::Haptics => write!(f, "haptics"),
            Capability::BackgroundTasks => write!(f, "background_tasks"),
        }
    }
}

/// Result type for capability operations
pub type CapabilityResult<T> = Result<T, CapabilityError>;

/// Errors that can occur when using capabilities
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityError {
    /// The capability is not supported on this platform
    NotSupported(Capability),
    /// Permission was denied by the user
    PermissionDenied(Capability),
    /// The capability is not available (e.g., no camera hardware)
    NotAvailable(Capability),
    /// A general error occurred
    Error(String),
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CapabilityError::NotSupported(cap) => {
                write!(f, "Capability '{}' is not supported on this platform", cap)
            }
            CapabilityError::PermissionDenied(cap) => {
                write!(f, "Permission denied for capability '{}'", cap)
            }
            CapabilityError::NotAvailable(cap) => {
                write!(f, "Capability '{}' is not available", cap)
            }
            CapabilityError::Error(msg) => write!(f, "Capability error: {}", msg),
        }
    }
}

impl std::error::Error for CapabilityError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_display() {
        assert_eq!(Capability::Filesystem.to_string(), "filesystem");
        assert_eq!(Capability::Camera.to_string(), "camera");
        assert_eq!(Capability::Location.to_string(), "location");
    }

    #[test]
    fn test_capability_error_display() {
        let err = CapabilityError::NotSupported(Capability::SystemTray);
        assert!(err.to_string().contains("system_tray"));
        assert!(err.to_string().contains("not supported"));
    }
}
