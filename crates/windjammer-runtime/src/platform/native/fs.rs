//! Native file system implementation
//!
//! Re-exports the existing windjammer-runtime fs module.

// Re-export all functions from the parent fs module
pub use crate::fs::*;

// Add any additional functions needed by std::fs API
pub fn read_file(path: String) -> Result<String, String> {
    read_to_string(path)
}

pub fn read_bytes(path: String) -> Result<Vec<u8>, String> {
    std::fs::read(&path).map_err(|e| format!("Failed to read file {}: {}", path, e))
}

pub fn write_file(path: String, content: String) -> Result<(), String> {
    write_string(path, &content)
}

pub fn file_exists(path: String) -> bool {
    exists(path)
}

pub fn create_directory(path: String) -> Result<(), String> {
    create_dir_all(path)
}

pub fn delete_file(path: String) -> Result<(), String> {
    remove_file(path)
}

pub fn list_directory(path: String) -> Result<Vec<FileEntry>, String> {
    use std::fs;

    let entries = fs::read_dir(&path).map_err(|e| format!("Failed to read directory: {}", e))?;

    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path_buf = entry.path();
        let name = path_buf
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        files.push(FileEntry {
            name,
            path: path_buf.to_string_lossy().to_string(),
            is_directory: path_buf.is_dir(),
        });
    }

    Ok(files)
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
}
