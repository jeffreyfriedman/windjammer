//! Asset loading and management

pub mod texture_loader;

pub use texture_loader::{
    Texture, TextureConfig, TextureFilter, TextureFormat, TextureHandle, TextureLoader,
    TextureWrap,
};

use std::collections::HashMap;
use std::path::Path;

/// Asset manager for loading and caching assets
pub struct AssetManager {
    assets: HashMap<String, Vec<u8>>,
}

impl AssetManager {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }

    pub fn load(&mut self, path: &Path) -> Result<Handle<Asset>, String> {
        let path_str = path.to_string_lossy().to_string();

        // Load the asset
        let data =
            std::fs::read(path).map_err(|e| format!("Failed to load asset {}: {}", path_str, e))?;

        self.assets.insert(path_str.clone(), data);

        Ok(Handle::new(path_str))
    }

    pub fn get(&self, handle: &Handle<Asset>) -> Option<&[u8]> {
        self.assets.get(&handle.path).map(|v| v.as_slice())
    }
}

impl Default for AssetManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Generic asset type
pub struct Asset {
    _placeholder: (),
}

/// Handle to an asset
pub struct Handle<T> {
    path: String,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Handle<T> {
    pub fn new(path: String) -> Self {
        Self {
            path,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_manager_creation() {
        let manager = AssetManager::new();
        assert_eq!(manager.assets.len(), 0);
    }
}
