//! File system operations
//!
//! Windjammer's `std::fs` module maps to these functions.

use std::path::Path;

/// Read entire file as a string
pub fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

/// Read entire file as bytes
pub fn read<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| e.to_string())
}

/// Write string to file
pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<(), String> {
    std::fs::write(path, contents).map_err(|e| e.to_string())
}

/// Check if path exists
pub fn exists<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().exists()
}

/// Check if path is a file
pub fn is_file<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().is_file()
}

/// Check if path is a directory
pub fn is_dir<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().is_dir()
}

/// Create a directory
pub fn create_dir<P: AsRef<Path>>(path: P) -> Result<(), String> {
    std::fs::create_dir(path).map_err(|e| e.to_string())
}

/// Create a directory and all parent directories
pub fn create_dir_all<P: AsRef<Path>>(path: P) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| e.to_string())
}

/// Remove a file
pub fn remove_file<P: AsRef<Path>>(path: P) -> Result<(), String> {
    std::fs::remove_file(path).map_err(|e| e.to_string())
}

/// Remove a directory
pub fn remove_dir<P: AsRef<Path>>(path: P) -> Result<(), String> {
    std::fs::remove_dir(path).map_err(|e| e.to_string())
}

/// Remove a directory and all its contents
pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> Result<(), String> {
    std::fs::remove_dir_all(path).map_err(|e| e.to_string())
}

/// Copy a file
pub fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<u64, String> {
    std::fs::copy(from, to).map_err(|e| e.to_string())
}

/// Rename a file or directory
pub fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<(), String> {
    std::fs::rename(from, to).map_err(|e| e.to_string())
}

/// List directory entries
pub fn read_dir<P: AsRef<Path>>(path: P) -> Result<Vec<String>, String> {
    let entries = std::fs::read_dir(path).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if let Some(path_str) = path.to_str() {
            result.push(path_str.to_string());
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_write() {
        let temp = std::env::temp_dir().join("windjammer_test.txt");
        let content = "Hello, Windjammer!";

        write(&temp, content).unwrap();
        let read_content = read_to_string(&temp).unwrap();

        assert_eq!(content, read_content);
        remove_file(&temp).unwrap();
    }

    #[test]
    fn test_exists() {
        let temp = std::env::temp_dir().join("windjammer_exists_test.txt");

        assert!(!exists(&temp));
        write(&temp, "test").unwrap();
        assert!(exists(&temp));
        assert!(is_file(&temp));
        assert!(!is_dir(&temp));

        remove_file(&temp).unwrap();
    }
}
