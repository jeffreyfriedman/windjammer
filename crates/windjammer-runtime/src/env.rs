//! Environment variable operations
//!
//! Windjammer's `std::env` module maps to these functions.

/// Get an environment variable
pub fn var(key: &str) -> Result<String, String> {
    crate::caps::env_read(key);
    std::env::var(key).map_err(|e| e.to_string())
}

/// Get an environment variable or return a default
pub fn get_or(key: &str, default: &str) -> String {
    crate::caps::env_read(key);
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Get an environment variable (optional)
pub fn get(key: &str) -> Option<String> {
    crate::caps::env_read(key);
    std::env::var(key).ok()
}

/// Set an environment variable
pub fn set_var(key: &str, value: &str) {
    crate::caps::env_write(key);
    std::env::set_var(key, value);
}

/// Remove an environment variable
pub fn remove_var(key: &str) {
    crate::caps::env_write(key);
    std::env::remove_var(key);
}

/// Set an environment variable (Windjammer `std::env::set`).
pub fn set(key: &str, value: &str) {
    set_var(key, value);
}

/// Remove an environment variable (Windjammer `std::env::remove`).
pub fn remove(key: &str) {
    remove_var(key);
}

/// The user's home directory (`$HOME`), if set.
pub fn home_dir() -> Option<String> {
    crate::caps::env_read("HOME");
    std::env::var("HOME").ok()
}

/// Get all environment variables
pub fn vars() -> Vec<(String, String)> {
    crate::caps::env_read_all();
    std::env::vars().collect()
}

/// Get current working directory
pub fn current_dir() -> Result<String, String> {
    std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

/// Get current executable path
pub fn current_exe() -> Result<String, String> {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

/// Get command line arguments
pub fn args() -> Vec<String> {
    std::env::args().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_get_var() {
        set_var("WINDJAMMER_TEST", "hello");
        assert_eq!(var("WINDJAMMER_TEST"), Ok("hello".to_string()));
        remove_var("WINDJAMMER_TEST");
        assert!(var("WINDJAMMER_TEST").is_err());
    }

    #[test]
    fn test_current_dir() {
        let dir = current_dir();
        assert!(dir.is_ok());
    }

    #[test]
    fn test_args() {
        let args = args();
        assert!(!args.is_empty());
    }

    #[test]
    fn set_remove_and_home_dir() {
        set("WINDJAMMER_TEST_SET_REMOVE", "v");
        assert_eq!(var("WINDJAMMER_TEST_SET_REMOVE").as_deref(), Ok("v"));
        remove("WINDJAMMER_TEST_SET_REMOVE");
        assert!(var("WINDJAMMER_TEST_SET_REMOVE").is_err());
        assert_eq!(home_dir(), std::env::var("HOME").ok());
    }
}
