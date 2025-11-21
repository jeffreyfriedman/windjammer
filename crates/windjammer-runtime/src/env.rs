//! Environment variable operations
//!
//! Windjammer's `std::env` module maps to these functions.

/// Get an environment variable
pub fn var(key: &str) -> Result<String, String> {
    std::env::var(key).map_err(|e| e.to_string())
}

/// Set an environment variable
pub fn set_var(key: &str, value: &str) {
    std::env::set_var(key, value);
}

/// Remove an environment variable
pub fn remove_var(key: &str) {
    std::env::remove_var(key);
}

/// Get all environment variables
pub fn vars() -> Vec<(String, String)> {
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
}
