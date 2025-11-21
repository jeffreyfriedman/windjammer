//! Path manipulation utilities
//!
//! Windjammer's path module provides a clean, ergonomic API for working with
//! file system paths. It wraps Rust's std::path but with simplified error handling.

use std::ffi::OsStr;
use std::path::{Path as StdPath, PathBuf as StdPathBuf};

/// Re-export std::path::Path for direct use
pub use std::path::Path;

/// Re-export std::path::PathBuf for direct use  
pub use std::path::PathBuf;

/// Create a new Path from a string
pub fn new(s: &str) -> &StdPath {
    StdPath::new(s)
}

/// Create a new PathBuf from a string
pub fn from_str(s: &str) -> StdPathBuf {
    StdPathBuf::from(s)
}

/// Get the file name from a path
pub fn file_name(path: &StdPath) -> Option<&str> {
    path.file_name().and_then(|s| s.to_str())
}

/// Get the file stem (name without extension)
pub fn file_stem(path: &StdPath) -> Option<&str> {
    path.file_stem().and_then(|s| s.to_str())
}

/// Get the file extension (works with &str, &String, &Path, &PathBuf)
pub fn extension<P: AsRef<StdPath>>(path: P) -> Option<String> {
    path.as_ref()
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

/// Get the parent directory
pub fn parent(path: &StdPath) -> Option<&StdPath> {
    path.parent()
}

/// Check if path is absolute
pub fn is_absolute(path: &StdPath) -> bool {
    path.is_absolute()
}

/// Check if path is relative
pub fn is_relative(path: &StdPath) -> bool {
    path.is_relative()
}

/// Check if path exists
pub fn exists(path: &StdPath) -> bool {
    path.exists()
}

/// Check if path is a file
pub fn is_file(path: &StdPath) -> bool {
    path.is_file()
}

/// Check if path is a directory
pub fn is_dir(path: &StdPath) -> bool {
    path.is_dir()
}

/// Join path segments
pub fn join(path: &StdPath, other: &str) -> StdPathBuf {
    path.join(other)
}

/// Convert path to string
pub fn to_string(path: &StdPath) -> Option<String> {
    path.to_str().map(String::from)
}

/// Convert path to string, lossy
pub fn to_string_lossy(path: &StdPath) -> String {
    path.to_string_lossy().into_owned()
}

/// Get components of the path as strings
pub fn components(path: &StdPath) -> Vec<String> {
    path.components()
        .filter_map(|c| c.as_os_str().to_str())
        .map(String::from)
        .collect()
}

/// Normalize a path (resolve . and ..)
pub fn canonicalize(path: &StdPath) -> Result<StdPathBuf, String> {
    path.canonicalize()
        .map_err(|e| format!("Failed to canonicalize path: {}", e))
}

/// Get the current working directory
pub fn current_dir() -> Result<StdPathBuf, String> {
    std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))
}

/// Check if a path has a specific extension
pub fn has_extension(path: &StdPath, ext: &str) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e == ext)
        .unwrap_or(false)
}

/// Strip prefix from path
pub fn strip_prefix<'a>(path: &'a StdPath, prefix: &StdPath) -> Option<&'a StdPath> {
    path.strip_prefix(prefix).ok()
}

/// Get the path as an OsStr
pub fn as_os_str(path: &StdPath) -> &OsStr {
    path.as_os_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_operations() {
        let path = new("/foo/bar/baz.txt");

        assert_eq!(file_name(path), Some("baz.txt"));
        assert_eq!(file_stem(path), Some("baz"));
        assert_eq!(extension(path), Some("txt".to_string()));
        assert!(is_absolute(path));

        let relative = new("foo/bar.txt");
        assert!(is_relative(relative));
    }

    #[test]
    fn test_path_join() {
        let base = new("/foo");
        let joined = join(base, "bar");
        assert_eq!(to_string(&joined), Some("/foo/bar".to_string()));
    }

    #[test]
    fn test_extension_check() {
        let path = new("file.wj");
        assert!(has_extension(path, "wj"));
        assert!(!has_extension(path, "rs"));
    }
}
