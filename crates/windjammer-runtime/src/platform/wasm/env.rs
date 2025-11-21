//! WASM environment implementation
//!
//! Uses localStorage for environment-like storage in the browser.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use web_sys::{console, window, Storage};

#[cfg(target_arch = "wasm32")]
fn get_local_storage() -> Option<Storage> {
    window()?.local_storage().ok()?
}

/// Get an environment variable (uses localStorage)
pub fn get(key: String) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        get_local_storage()?.get_item(&key).ok()?
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

/// Get an environment variable or return default (uses localStorage)
pub fn get_or(key: String, default: String) -> String {
    get(key).unwrap_or(default)
}

/// Set an environment variable (uses localStorage)
pub fn set(key: String, value: String) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = get_local_storage() {
            if let Err(_) = storage.set_item(&key, &value) {
                console::warn_1(&format!("Failed to set localStorage item: {}", key).into());
            }
        }
    }
}

/// Remove an environment variable (uses localStorage)
pub fn remove(key: String) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = get_local_storage() {
            if let Err(_) = storage.remove_item(&key) {
                console::warn_1(&format!("Failed to remove localStorage item: {}", key).into());
            }
        }
    }
}

/// Get current directory (returns "/" in browser)
pub fn current_dir() -> String {
    "/".to_string()
}

/// Get all environment variables (returns all localStorage items)
pub fn vars() -> Vec<(String, String)> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut vars = Vec::new();
        if let Some(storage) = get_local_storage() {
            if let Ok(len) = storage.length() {
                for i in 0..len {
                    if let Ok(Some(key)) = storage.key(i) {
                        if let Ok(Some(value)) = storage.get_item(&key) {
                            vars.push((key, value));
                        }
                    }
                }
            }
        }
        vars
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Vec::new()
    }
}

/// Get home directory (not available in browser)
pub fn home_dir() -> Option<String> {
    None
}

/// Get temp directory (returns "/tmp" virtual path in browser)
pub fn temp_dir() -> String {
    "/tmp".to_string()
}

// Note: In the browser:
// - "Environment variables" are stored in localStorage
// - They persist across page reloads
// - They are scoped to the origin (domain)
// - They have a storage limit (usually 5-10MB)
