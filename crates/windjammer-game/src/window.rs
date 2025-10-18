//! Window creation and management

/// Window configuration
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub vsync: bool,
}

impl WindowConfig {
    pub fn new(title: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            title: title.into(),
            width,
            height,
            resizable: true,
            vsync: true,
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self::new("Windjammer Game", 1280, 720)
    }
}

/// Window handle
pub struct Window {
    config: WindowConfig,
}

impl Window {
    pub fn new(config: WindowConfig) -> Self {
        Self { config }
    }

    pub fn title(&self) -> &str {
        &self.config.title
    }

    pub fn width(&self) -> u32 {
        self.config.width
    }

    pub fn height(&self) -> u32 {
        self.config.height
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.config.width as f32 / self.config.height as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_config() {
        let config = WindowConfig::new("Test", 800, 600);
        assert_eq!(config.title, "Test");
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
    }

    #[test]
    fn test_window_creation() {
        let config = WindowConfig::default();
        let window = Window::new(config);
        assert_eq!(window.width(), 1280);
        assert_eq!(window.height(), 720);
    }
}
