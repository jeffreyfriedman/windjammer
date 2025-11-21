//! Native environment implementation
//!
//! Re-exports the existing windjammer-runtime env module.

// Re-export all functions from the parent env module
pub use crate::env::*;

// Add any additional functions needed by std::env API
pub fn get(key: String) -> Option<String> {
    var(&key).ok()
}

pub fn get_or(key: String, default: String) -> String {
    var(&key).unwrap_or(default)
}

pub fn set(key: String, value: String) {
    std::env::set_var(key, value);
}

pub fn remove(key: String) {
    std::env::remove_var(key);
}

pub fn vars() -> Vec<(String, String)> {
    std::env::vars().collect()
}

pub fn home_dir() -> Option<String> {
    dirs::home_dir().map(|p| p.to_string_lossy().to_string())
}

pub fn temp_dir() -> String {
    std::env::temp_dir().to_string_lossy().to_string()
}
