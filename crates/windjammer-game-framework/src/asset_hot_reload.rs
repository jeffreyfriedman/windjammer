//! Asset Hot-Reload System
//!
//! Provides automatic asset reloading for rapid iteration during development.
//! Watches asset directories for changes and automatically reloads modified assets.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// Asset hot-reload system
pub struct AssetHotReload {
    /// Watched asset paths
    watched_paths: Vec<PathBuf>,
    
    /// Asset metadata (path -> last modified time)
    asset_metadata: HashMap<PathBuf, SystemTime>,
    
    /// Reload callbacks (asset type -> callback)
    reload_callbacks: HashMap<String, Box<dyn Fn(&Path) + Send + Sync>>,
    
    /// Polling interval (for file watching)
    poll_interval: Duration,
    
    /// Last poll time
    last_poll: SystemTime,
    
    /// Enable/disable hot reload
    enabled: bool,
}

impl AssetHotReload {
    /// Create a new asset hot-reload system
    pub fn new() -> Self {
        Self {
            watched_paths: Vec::new(),
            asset_metadata: HashMap::new(),
            reload_callbacks: HashMap::new(),
            poll_interval: Duration::from_millis(500), // Poll every 500ms
            last_poll: SystemTime::now(),
            enabled: true,
        }
    }
    
    /// Add a directory to watch for changes
    pub fn watch_directory(&mut self, path: PathBuf) {
        if !self.watched_paths.contains(&path) {
            self.watched_paths.push(path.clone());
            
            // Scan directory and record initial metadata
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_file() {
                            if let Ok(modified) = metadata.modified() {
                                self.asset_metadata.insert(entry.path(), modified);
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Add a specific file to watch
    pub fn watch_file(&mut self, path: PathBuf) {
        if let Ok(metadata) = std::fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                self.asset_metadata.insert(path, modified);
            }
        }
    }
    
    /// Register a reload callback for an asset type
    pub fn register_callback<F>(&mut self, asset_type: String, callback: F)
    where
        F: Fn(&Path) + Send + Sync + 'static,
    {
        self.reload_callbacks.insert(asset_type, Box::new(callback));
    }
    
    /// Set polling interval
    pub fn set_poll_interval(&mut self, interval: Duration) {
        self.poll_interval = interval;
    }
    
    /// Enable hot reload
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    /// Disable hot reload
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    /// Check for asset changes and reload if necessary
    pub fn update(&mut self) {
        if !self.enabled {
            return;
        }
        
        let now = SystemTime::now();
        if now.duration_since(self.last_poll).unwrap_or(Duration::ZERO) < self.poll_interval {
            return;
        }
        
        self.last_poll = now;
        
        // Check all watched files
        let mut changed_assets = Vec::new();
        
        for (path, last_modified) in &self.asset_metadata {
            if let Ok(metadata) = std::fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    if modified > *last_modified {
                        changed_assets.push((path.clone(), modified));
                    }
                }
            }
        }
        
        // Update metadata and trigger callbacks
        for (path, new_modified) in changed_assets {
            self.asset_metadata.insert(path.clone(), new_modified);
            
            // Determine asset type from extension
            if let Some(extension) = path.extension() {
                if let Some(ext_str) = extension.to_str() {
                    let asset_type = ext_str.to_lowercase();
                    
                    // Call registered callback
                    if let Some(callback) = self.reload_callbacks.get(&asset_type) {
                        callback(&path);
                    }
                }
            }
        }
        
        // Scan watched directories for new files
        for watched_path in &self.watched_paths {
            if let Ok(entries) = std::fs::read_dir(watched_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !self.asset_metadata.contains_key(&path) {
                        if let Ok(metadata) = entry.metadata() {
                            if metadata.is_file() {
                                if let Ok(modified) = metadata.modified() {
                                    self.asset_metadata.insert(path.clone(), modified);
                                    
                                    // Trigger callback for new file
                                    if let Some(extension) = path.extension() {
                                        if let Some(ext_str) = extension.to_str() {
                                            let asset_type = ext_str.to_lowercase();
                                            if let Some(callback) = self.reload_callbacks.get(&asset_type) {
                                                callback(&path);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Get list of watched paths
    pub fn watched_paths(&self) -> &[PathBuf] {
        &self.watched_paths
    }
    
    /// Get number of tracked assets
    pub fn tracked_asset_count(&self) -> usize {
        self.asset_metadata.len()
    }
}

impl Default for AssetHotReload {
    fn default() -> Self {
        Self::new()
    }
}

/// Asset reload event
#[derive(Debug, Clone)]
pub struct AssetReloadEvent {
    /// Path to the reloaded asset
    pub path: PathBuf,
    
    /// Asset type (file extension)
    pub asset_type: String,
    
    /// Timestamp of reload
    pub timestamp: SystemTime,
}

/// Asset hot-reload manager (thread-safe)
pub struct AssetHotReloadManager {
    inner: Arc<Mutex<AssetHotReload>>,
}

impl AssetHotReloadManager {
    /// Create a new hot-reload manager
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(AssetHotReload::new())),
        }
    }
    
    /// Watch a directory
    pub fn watch_directory(&self, path: PathBuf) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.watch_directory(path);
        }
    }
    
    /// Watch a file
    pub fn watch_file(&self, path: PathBuf) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.watch_file(path);
        }
    }
    
    /// Register a reload callback
    pub fn register_callback<F>(&self, asset_type: String, callback: F)
    where
        F: Fn(&Path) + Send + Sync + 'static,
    {
        if let Ok(mut inner) = self.inner.lock() {
            inner.register_callback(asset_type, callback);
        }
    }
    
    /// Update (check for changes)
    pub fn update(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.update();
        }
    }
    
    /// Enable hot reload
    pub fn enable(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.enable();
        }
    }
    
    /// Disable hot reload
    pub fn disable(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.disable();
        }
    }
    
    /// Get tracked asset count
    pub fn tracked_asset_count(&self) -> usize {
        if let Ok(inner) = self.inner.lock() {
            inner.tracked_asset_count()
        } else {
            0
        }
    }
}

impl Default for AssetHotReloadManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for AssetHotReloadManager {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Helper for common asset types
pub mod asset_types {
    /// Texture extensions
    pub const TEXTURES: &[&str] = &["png", "jpg", "jpeg", "bmp", "tga", "dds"];
    
    /// Model extensions
    pub const MODELS: &[&str] = &["gltf", "glb", "obj", "fbx"];
    
    /// Audio extensions
    pub const AUDIO: &[&str] = &["wav", "mp3", "ogg", "flac"];
    
    /// Shader extensions
    pub const SHADERS: &[&str] = &["wgsl", "glsl", "hlsl", "spv"];
    
    /// Script extensions
    pub const SCRIPTS: &[&str] = &["wj", "lua", "py"];
    
    /// Check if path is a texture
    pub fn is_texture(path: &std::path::Path) -> bool {
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return TEXTURES.contains(&ext_str.to_lowercase().as_str());
            }
        }
        false
    }
    
    /// Check if path is a model
    pub fn is_model(path: &std::path::Path) -> bool {
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return MODELS.contains(&ext_str.to_lowercase().as_str());
            }
        }
        false
    }
    
    /// Check if path is audio
    pub fn is_audio(path: &std::path::Path) -> bool {
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return AUDIO.contains(&ext_str.to_lowercase().as_str());
            }
        }
        false
    }
    
    /// Check if path is a shader
    pub fn is_shader(path: &std::path::Path) -> bool {
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return SHADERS.contains(&ext_str.to_lowercase().as_str());
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    
    #[test]
    fn test_hot_reload_creation() {
        let hot_reload = AssetHotReload::new();
        assert!(hot_reload.enabled);
        assert_eq!(hot_reload.watched_paths().len(), 0);
        assert_eq!(hot_reload.tracked_asset_count(), 0);
    }
    
    #[test]
    fn test_watch_directory() {
        let mut hot_reload = AssetHotReload::new();
        let temp_dir = std::env::temp_dir().join("windjammer_test_assets");
        
        // Create temp directory
        let _ = fs::create_dir_all(&temp_dir);
        
        hot_reload.watch_directory(temp_dir.clone());
        assert_eq!(hot_reload.watched_paths().len(), 1);
        
        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
    
    #[test]
    fn test_asset_type_detection() {
        assert!(asset_types::is_texture(Path::new("texture.png")));
        assert!(asset_types::is_model(Path::new("model.gltf")));
        assert!(asset_types::is_audio(Path::new("sound.wav")));
        assert!(asset_types::is_shader(Path::new("shader.wgsl")));
        
        assert!(!asset_types::is_texture(Path::new("model.gltf")));
    }
    
    #[test]
    fn test_hot_reload_manager() {
        let manager = AssetHotReloadManager::new();
        assert_eq!(manager.tracked_asset_count(), 0);
        
        manager.enable();
        manager.disable();
    }
}

