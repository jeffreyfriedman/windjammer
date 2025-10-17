//! Implementation of native platform capabilities

use super::capabilities::{Capability, CapabilityError, CapabilityResult};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Filesystem API
pub struct Filesystem {
    allowed_paths: Arc<Mutex<Vec<String>>>,
}

impl Filesystem {
    pub fn new() -> Self {
        Self {
            allowed_paths: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Request permission to access a path
    pub fn request_permission(&self, path: impl AsRef<Path>) -> CapabilityResult<()> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let mut allowed = self.allowed_paths.lock().unwrap();
        if !allowed.contains(&path_str) {
            allowed.push(path_str);
        }
        Ok(())
    }

    /// Read file contents
    pub fn read_text(&self, path: impl AsRef<Path>) -> CapabilityResult<String> {
        self.check_permission(path.as_ref())?;
        std::fs::read_to_string(path)
            .map_err(|e| CapabilityError::Error(format!("Read error: {}", e)))
    }

    /// Write text to file
    pub fn write_text(&self, path: impl AsRef<Path>, content: &str) -> CapabilityResult<()> {
        self.check_permission(path.as_ref())?;
        std::fs::write(path, content)
            .map_err(|e| CapabilityError::Error(format!("Write error: {}", e)))
    }

    /// Read binary file
    pub fn read_binary(&self, path: impl AsRef<Path>) -> CapabilityResult<Vec<u8>> {
        self.check_permission(path.as_ref())?;
        std::fs::read(path).map_err(|e| CapabilityError::Error(format!("Read error: {}", e)))
    }

    /// Write binary to file
    pub fn write_binary(&self, path: impl AsRef<Path>, data: &[u8]) -> CapabilityResult<()> {
        self.check_permission(path.as_ref())?;
        std::fs::write(path, data)
            .map_err(|e| CapabilityError::Error(format!("Write error: {}", e)))
    }

    /// Check if file exists
    pub fn exists(&self, path: impl AsRef<Path>) -> bool {
        path.as_ref().exists()
    }

    /// List directory contents
    pub fn list_dir(&self, path: impl AsRef<Path>) -> CapabilityResult<Vec<String>> {
        self.check_permission(path.as_ref())?;
        let entries = std::fs::read_dir(path)
            .map_err(|e| CapabilityError::Error(format!("Read directory error: {}", e)))?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        Ok(entries)
    }

    fn check_permission(&self, path: &Path) -> CapabilityResult<()> {
        let path_str = path.to_string_lossy().to_string();
        let allowed = self.allowed_paths.lock().unwrap();

        // Check if path or any parent is in allowed list
        for allowed_path in allowed.iter() {
            if path_str.starts_with(allowed_path) {
                return Ok(());
            }
        }

        Err(CapabilityError::PermissionDenied(Capability::Filesystem))
    }
}

impl Default for Filesystem {
    fn default() -> Self {
        Self::new()
    }
}

/// GPS Location
#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
    pub accuracy: f64,
    pub timestamp: u64,
}

/// GPS API
pub struct GPS {
    current_location: Arc<Mutex<Option<Location>>>,
    enabled: Arc<Mutex<bool>>,
}

impl GPS {
    pub fn new() -> Self {
        Self {
            current_location: Arc::new(Mutex::new(None)),
            enabled: Arc::new(Mutex::new(false)),
        }
    }

    /// Request GPS permission and enable
    pub fn enable(&self) -> CapabilityResult<()> {
        *self.enabled.lock().unwrap() = true;
        Ok(())
    }

    /// Get current location
    pub fn get_location(&self) -> CapabilityResult<Location> {
        if !*self.enabled.lock().unwrap() {
            return Err(CapabilityError::PermissionDenied(Capability::Location));
        }

        self.current_location
            .lock()
            .unwrap()
            .ok_or(CapabilityError::NotAvailable(Capability::Location))
    }

    /// Set location (for testing or simulation)
    pub fn set_location(&self, location: Location) {
        *self.current_location.lock().unwrap() = Some(location);
    }

    /// Check if GPS is enabled
    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock().unwrap()
    }
}

impl Default for GPS {
    fn default() -> Self {
        Self::new()
    }
}

/// Camera image data
#[derive(Debug, Clone)]
pub struct CameraImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub format: ImageFormat,
}

/// Image format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    RGB,
    RGBA,
    JPEG,
    PNG,
}

/// Camera API
pub struct Camera {
    enabled: Arc<Mutex<bool>>,
    captured_images: Arc<Mutex<Vec<CameraImage>>>,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(Mutex::new(false)),
            captured_images: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Request camera permission
    pub fn request_permission(&self) -> CapabilityResult<()> {
        *self.enabled.lock().unwrap() = true;
        Ok(())
    }

    /// Capture an image
    pub fn capture(&self) -> CapabilityResult<CameraImage> {
        if !*self.enabled.lock().unwrap() {
            return Err(CapabilityError::PermissionDenied(Capability::Camera));
        }

        // In real implementation, would access device camera
        // For now, return a placeholder
        let image = CameraImage {
            width: 640,
            height: 480,
            data: vec![0; 640 * 480 * 3],
            format: ImageFormat::RGB,
        };

        self.captured_images.lock().unwrap().push(image.clone());
        Ok(image)
    }

    /// Get captured images
    pub fn get_captured_images(&self) -> Vec<CameraImage> {
        self.captured_images.lock().unwrap().clone()
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

/// Clipboard API
pub struct Clipboard {
    content: Arc<Mutex<String>>,
}

impl Clipboard {
    pub fn new() -> Self {
        Self {
            content: Arc::new(Mutex::new(String::new())),
        }
    }

    /// Read clipboard text
    pub fn read_text(&self) -> CapabilityResult<String> {
        Ok(self.content.lock().unwrap().clone())
    }

    /// Write text to clipboard
    pub fn write_text(&self, text: &str) -> CapabilityResult<()> {
        *self.content.lock().unwrap() = text.to_string();
        Ok(())
    }

    /// Check if clipboard has content
    pub fn has_content(&self) -> bool {
        !self.content.lock().unwrap().is_empty()
    }
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::new()
    }
}

/// Notification
#[derive(Debug, Clone)]
pub struct Notification {
    pub title: String,
    pub body: String,
    pub icon: Option<String>,
}

/// Notifications API
pub struct Notifications {
    enabled: Arc<Mutex<bool>>,
    sent_notifications: Arc<Mutex<Vec<Notification>>>,
}

impl Notifications {
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(Mutex::new(false)),
            sent_notifications: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Request notification permission
    pub fn request_permission(&self) -> CapabilityResult<()> {
        *self.enabled.lock().unwrap() = true;
        Ok(())
    }

    /// Send a notification
    pub fn send(&self, notification: Notification) -> CapabilityResult<()> {
        if !*self.enabled.lock().unwrap() {
            return Err(CapabilityError::PermissionDenied(Capability::Notifications));
        }

        self.sent_notifications.lock().unwrap().push(notification);
        Ok(())
    }

    /// Get sent notifications (for testing)
    pub fn get_sent(&self) -> Vec<Notification> {
        self.sent_notifications.lock().unwrap().clone()
    }
}

impl Default for Notifications {
    fn default() -> Self {
        Self::new()
    }
}

/// Accelerometer data
#[derive(Debug, Clone, Copy)]
pub struct AccelerometerData {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub timestamp: u64,
}

/// Accelerometer API
pub struct Accelerometer {
    enabled: Arc<Mutex<bool>>,
    current_data: Arc<Mutex<Option<AccelerometerData>>>,
}

impl Accelerometer {
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(Mutex::new(false)),
            current_data: Arc::new(Mutex::new(None)),
        }
    }

    /// Start accelerometer
    pub fn start(&self) -> CapabilityResult<()> {
        *self.enabled.lock().unwrap() = true;
        Ok(())
    }

    /// Stop accelerometer
    pub fn stop(&self) {
        *self.enabled.lock().unwrap() = false;
    }

    /// Get current data
    pub fn get_data(&self) -> CapabilityResult<AccelerometerData> {
        if !*self.enabled.lock().unwrap() {
            return Err(CapabilityError::NotAvailable(Capability::Haptics));
        }

        self.current_data
            .lock()
            .unwrap()
            .ok_or(CapabilityError::NotAvailable(Capability::Haptics))
    }

    /// Set data (for testing)
    pub fn set_data(&self, data: AccelerometerData) {
        *self.current_data.lock().unwrap() = Some(data);
    }
}

impl Default for Accelerometer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_permission() {
        let fs = Filesystem::new();
        assert!(fs.request_permission("/tmp").is_ok());
    }

    #[test]
    fn test_filesystem_read_write() {
        let fs = Filesystem::new();
        let path = "/tmp/test_windjammer.txt";

        fs.request_permission("/tmp").unwrap();
        fs.write_text(path, "Hello, Windjammer!").unwrap();
        let content = fs.read_text(path).unwrap();

        assert_eq!(content, "Hello, Windjammer!");

        // Clean up
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_filesystem_permission_denied() {
        let fs = Filesystem::new();
        let result = fs.read_text("/etc/passwd");
        assert!(matches!(
            result,
            Err(CapabilityError::PermissionDenied(Capability::Filesystem))
        ));
    }

    #[test]
    fn test_gps_enable() {
        let gps = GPS::new();
        assert!(!gps.is_enabled());

        gps.enable().unwrap();
        assert!(gps.is_enabled());
    }

    #[test]
    fn test_gps_location() {
        let gps = GPS::new();
        gps.enable().unwrap();

        let location = Location {
            latitude: 37.7749,
            longitude: -122.4194,
            altitude: Some(10.0),
            accuracy: 5.0,
            timestamp: 1234567890,
        };

        gps.set_location(location);
        let retrieved = gps.get_location().unwrap();

        assert_eq!(retrieved.latitude, 37.7749);
        assert_eq!(retrieved.longitude, -122.4194);
    }

    #[test]
    fn test_camera_permission() {
        let camera = Camera::new();
        assert!(camera.request_permission().is_ok());
    }

    #[test]
    fn test_camera_capture() {
        let camera = Camera::new();
        camera.request_permission().unwrap();

        let image = camera.capture().unwrap();
        assert_eq!(image.width, 640);
        assert_eq!(image.height, 480);
        assert_eq!(camera.get_captured_images().len(), 1);
    }

    #[test]
    fn test_clipboard() {
        let clipboard = Clipboard::new();
        assert!(!clipboard.has_content());

        clipboard.write_text("Test content").unwrap();
        assert!(clipboard.has_content());

        let content = clipboard.read_text().unwrap();
        assert_eq!(content, "Test content");
    }

    #[test]
    fn test_notifications() {
        let notifications = Notifications::new();
        notifications.request_permission().unwrap();

        let notification = Notification {
            title: "Test".to_string(),
            body: "This is a test".to_string(),
            icon: None,
        };

        notifications.send(notification).unwrap();
        assert_eq!(notifications.get_sent().len(), 1);
    }

    #[test]
    fn test_accelerometer() {
        let accel = Accelerometer::new();
        accel.start().unwrap();

        let data = AccelerometerData {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            timestamp: 1234567890,
        };

        accel.set_data(data);
        let retrieved = accel.get_data().unwrap();

        assert_eq!(retrieved.x, 1.0);
        assert_eq!(retrieved.y, 2.0);
        assert_eq!(retrieved.z, 3.0);
    }
}
