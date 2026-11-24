//! File system operations
//!
//! Windjammer's `std::fs` module maps to these functions.

use std::path::Path;

/// Read entire file as a string
pub fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String, String> {
    use std::io::Read;
    let mut file = crate::caps::open_read(path).map_err(|e| e.to_string())?;
    let mut s = String::new();
    file.read_to_string(&mut s).map_err(|e| e.to_string())?;
    Ok(s)
}

/// Read entire file as bytes
pub fn read<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut file = crate::caps::open_read(path).map_err(|e| e.to_string())?;
    let mut b = Vec::new();
    file.read_to_end(&mut b).map_err(|e| e.to_string())?;
    Ok(b)
}

/// Alias for read() for clarity  
pub fn read_bytes<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, String> {
    read(path)
}

/// Write to file (accepts bytes or strings)
pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<(), String> {
    use std::io::Write;
    let mut f = crate::caps::open_write(path, false).map_err(|e| e.to_string())?;
    f.write_all(contents.as_ref()).map_err(|e| e.to_string())
}

/// Write bytes to a file
pub fn write_bytes<P: AsRef<Path>>(path: P, contents: &[u8]) -> Result<(), String> {
    write(path, contents)
}

/// The absolute, symlink-free form of a path
pub fn canonicalize<P: AsRef<Path>>(path: P) -> Result<String, String> {
    crate::caps::fs_read(&path);
    std::fs::canonicalize(path).map(|p| p.to_string_lossy().to_string()).map_err(|e| e.to_string())
}

/// Change the working directory (changes what every relative path means, so it is an environment write)
pub fn set_current_dir<P: AsRef<Path>>(path: P) -> Result<(), String> {
    crate::caps::env_write("cwd");
    std::env::set_current_dir(path).map_err(|e| e.to_string())
}

/// What `fs::metadata` knows about a path
#[derive(Debug, Clone)]
pub struct Metadata {
    len: u64,
    file: bool,
    dir: bool,
    readonly: bool,
}

impl Metadata {
    pub fn size(&self) -> u64 {
        self.len
    }
    pub fn len(&self) -> u64 {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn is_file(&self) -> bool {
        self.file
    }
    pub fn is_dir(&self) -> bool {
        self.dir
    }
    pub fn is_readonly(&self) -> bool {
        self.readonly
    }
}

/// Size and kind of a file or directory
pub fn metadata<P: AsRef<Path>>(path: P) -> Result<Metadata, String> {
    crate::caps::fs_read(&path);
    std::fs::metadata(path)
        .map(|m| Metadata { len: m.len(), file: m.is_file(), dir: m.is_dir(), readonly: m.permissions().readonly() })
        .map_err(|e| e.to_string())
}

/// Append to a file, creating it when missing (accepts bytes or strings)
pub fn append<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<(), String> {
    use std::io::Write;
    let mut f = crate::caps::open_write(path, true).map_err(|e| e.to_string())?;
    f.write_all(contents.as_ref()).map_err(|e| e.to_string())
}

/// Write string to file (helper for common case)
pub fn write_string<P: AsRef<Path>>(path: P, contents: &str) -> Result<(), String> {
    write(path, contents.as_bytes())
}

/// Check if path exists
pub fn exists<P: AsRef<Path>>(path: P) -> bool {
    crate::caps::fs_read(&path);
    path.as_ref().exists()
}

/// Check if path is a file
pub fn is_file<P: AsRef<Path>>(path: P) -> bool {
    crate::caps::fs_read(&path);
    path.as_ref().is_file()
}

/// Check if path is a directory
pub fn is_dir<P: AsRef<Path>>(path: P) -> bool {
    crate::caps::fs_read(&path);
    path.as_ref().is_dir()
}

/// Create a directory
pub fn create_dir<P: AsRef<Path>>(path: P) -> Result<(), String> {
    crate::caps::entry_op(path, crate::caps_core::grant::EntryOp::MakeDir).map_err(|e| e.to_string())
}

/// Create a directory and all parent directories
pub fn create_dir_all<P: AsRef<Path>>(path: P) -> Result<(), String> {
    crate::caps::dir_all(path, crate::caps_core::grant::DirAllOp::Create).map_err(|e| e.to_string())
}

/// Remove a file
pub fn remove_file<P: AsRef<Path>>(path: P) -> Result<(), String> {
    crate::caps::entry_op(path, crate::caps_core::grant::EntryOp::RemoveFile).map_err(|e| e.to_string())
}

/// Remove a directory
pub fn remove_dir<P: AsRef<Path>>(path: P) -> Result<(), String> {
    crate::caps::entry_op(path, crate::caps_core::grant::EntryOp::RemoveDir).map_err(|e| e.to_string())
}

/// Remove a directory and all its contents
pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> Result<(), String> {
    crate::caps::dir_all(path, crate::caps_core::grant::DirAllOp::Remove).map_err(|e| e.to_string())
}

/// Copy a file (Windjammer `std::fs::copy(from, to) -> Result<(), string>`: the declaration is the contract, the byte
/// count is not part of it)
pub fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<(), String> {
    crate::caps::copy(from, to).map(|_| ()).map_err(|e| e.to_string())
}

/// Rename a file or directory
pub fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<(), String> {
    crate::caps::rename(from, to).map_err(|e| e.to_string())
}

/// Directory entry wrapper
#[derive(Debug, Clone)]
pub struct DirEntry {
    path: String,
}

impl DirEntry {
    /// Get the full path to this entry
    pub fn path(&self) -> String {
        self.path.clone()
    }

    /// Get the file name (last component of path)
    pub fn file_name(&self) -> String {
        Path::new(&self.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    }

    /// Check if this entry is a file
    pub fn is_file(&self) -> bool {
        Path::new(&self.path).is_file()
    }

    /// Check if this entry is a directory
    pub fn is_dir(&self) -> bool {
        Path::new(&self.path).is_dir()
    }
}

/// List directory entries
pub fn read_dir<P: AsRef<Path>>(path: P) -> Result<Vec<DirEntry>, String> {
    crate::caps::fs_read(&path);
    let entries = std::fs::read_dir(path).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if let Some(path_str) = path.to_str() {
            result.push(DirEntry {
                path: path_str.to_string(),
            });
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_creates_then_extends() {
        let temp = std::env::temp_dir().join(format!("windjammer_append_{}.txt", std::process::id()));
        let _ = std::fs::remove_file(&temp);
        append(&temp, "a\n").unwrap();
        append(&temp, "b\n").unwrap();
        assert_eq!(read_to_string(&temp).unwrap(), "a\nb\n");
        remove_file(&temp).unwrap();
    }

    #[test]
    fn test_metadata_canonicalize_and_bytes() {
        let temp = std::env::temp_dir().join(format!("windjammer_meta_{}.bin", std::process::id()));
        write_bytes(&temp, &[1, 2, 3]).unwrap();
        let m = metadata(&temp).unwrap();
        assert!(m.len() == 3 && m.is_file() && !m.is_dir());
        assert!(canonicalize(&temp).unwrap().ends_with(temp.file_name().unwrap().to_str().unwrap()));
        assert!(metadata(temp.with_extension("missing")).is_err());
        remove_file(&temp).unwrap();
    }

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
