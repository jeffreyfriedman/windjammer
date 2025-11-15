/// Native implementation of std::storage using files
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub type StorageResult<T> = Result<T, String>;

/// Storage backend types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageBackend {
    Local,      // Local file storage
    Session,    // Temporary file storage
    Persistent, // Database storage (SQLite)
}

/// Get the storage directory for a given backend
fn get_storage_dir(backend: StorageBackend) -> StorageResult<PathBuf> {
    let base_dir =
        dirs::data_local_dir().ok_or_else(|| "Failed to get local data directory".to_string())?;

    let app_dir = base_dir.join("windjammer");

    let storage_dir = match backend {
        StorageBackend::Local => app_dir.join("storage"),
        StorageBackend::Session => std::env::temp_dir().join("windjammer_session"),
        StorageBackend::Persistent => app_dir.join("persistent"),
    };

    // Create directory if it doesn't exist
    fs::create_dir_all(&storage_dir)
        .map_err(|e| format!("Failed to create storage directory: {}", e))?;

    Ok(storage_dir)
}

/// Get the file path for a key
fn get_key_path(key: &str, backend: StorageBackend) -> StorageResult<PathBuf> {
    let storage_dir = get_storage_dir(backend)?;
    // Sanitize key to prevent directory traversal
    let safe_key = key.replace(['/', '\\'], "_").replace("..", "_");
    Ok(storage_dir.join(safe_key))
}

/// Store a key-value pair
pub fn set(key: String, value: String) -> StorageResult<()> {
    set_with_backend(key, value, StorageBackend::Local)
}

/// Get a value by key
pub fn get(key: String) -> StorageResult<Option<String>> {
    get_with_backend(key, StorageBackend::Local)
}

/// Remove a key
pub fn remove(key: String) -> StorageResult<()> {
    remove_with_backend(key, StorageBackend::Local)
}

/// Clear all storage
pub fn clear() -> StorageResult<()> {
    clear_with_backend(StorageBackend::Local)
}

/// List all keys
pub fn keys() -> StorageResult<Vec<String>> {
    keys_with_backend(StorageBackend::Local)
}

/// Check if a key exists
pub fn has(key: String) -> bool {
    get(key).unwrap_or(None).is_some()
}

// Backend-specific functions

/// Store with specific backend
pub fn set_with_backend(key: String, value: String, backend: StorageBackend) -> StorageResult<()> {
    let path = get_key_path(&key, backend)?;
    fs::write(&path, value).map_err(|e| format!("Failed to write storage file: {}", e))?;
    Ok(())
}

/// Get with specific backend
pub fn get_with_backend(key: String, backend: StorageBackend) -> StorageResult<Option<String>> {
    let path = get_key_path(&key, backend)?;
    if !path.exists() {
        return Ok(None);
    }
    let value =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read storage file: {}", e))?;
    Ok(Some(value))
}

/// Remove with specific backend
pub fn remove_with_backend(key: String, backend: StorageBackend) -> StorageResult<()> {
    let path = get_key_path(&key, backend)?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Failed to remove storage file: {}", e))?;
    }
    Ok(())
}

/// Clear with specific backend
pub fn clear_with_backend(backend: StorageBackend) -> StorageResult<()> {
    let storage_dir = get_storage_dir(backend)?;
    if storage_dir.exists() {
        fs::remove_dir_all(&storage_dir)
            .map_err(|e| format!("Failed to clear storage directory: {}", e))?;
        fs::create_dir_all(&storage_dir)
            .map_err(|e| format!("Failed to recreate storage directory: {}", e))?;
    }
    Ok(())
}

/// Keys with specific backend
pub fn keys_with_backend(backend: StorageBackend) -> StorageResult<Vec<String>> {
    let storage_dir = get_storage_dir(backend)?;
    let mut keys = Vec::new();

    if storage_dir.exists() {
        let entries = fs::read_dir(&storage_dir)
            .map_err(|e| format!("Failed to read storage directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            if let Some(name) = entry.file_name().to_str() {
                keys.push(name.to_string());
            }
        }
    }

    Ok(keys)
}

// Structured storage (JSON)

/// Store a JSON-serializable value
pub fn set_json<T: serde::Serialize>(key: String, value: T) -> StorageResult<()> {
    let json =
        serde_json::to_string(&value).map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
    set(key, json)
}

/// Get a JSON-serializable value
pub fn get_json<T: serde::de::DeserializeOwned>(key: String) -> StorageResult<Option<T>> {
    match get(key)? {
        Some(json) => {
            let value = serde_json::from_str(&json)
                .map_err(|e| format!("Failed to deserialize from JSON: {}", e))?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

// Binary storage

/// Store binary data
pub fn set_bytes(key: String, data: Vec<u8>) -> StorageResult<()> {
    let path = get_key_path(&key, StorageBackend::Local)?;
    fs::write(&path, data).map_err(|e| format!("Failed to write binary data: {}", e))?;
    Ok(())
}

/// Get binary data
pub fn get_bytes(key: String) -> StorageResult<Option<Vec<u8>>> {
    let path = get_key_path(&key, StorageBackend::Local)?;
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read(&path).map_err(|e| format!("Failed to read binary data: {}", e))?;
    Ok(Some(data))
}

// Cache management with TTL

/// Metadata for TTL storage
#[derive(serde::Serialize, serde::Deserialize)]
struct TtlMetadata {
    value: String,
    expires_at: u64, // Unix timestamp
}

/// Set with expiration (seconds)
pub fn set_with_ttl(key: String, value: String, ttl: i32) -> StorageResult<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Failed to get current time: {}", e))?
        .as_secs();

    let expires_at = now + ttl as u64;

    let metadata = TtlMetadata { value, expires_at };

    set_json(format!("{}_ttl", key), metadata)
}

/// Get with automatic expiration check
pub fn get_with_ttl(key: String) -> StorageResult<Option<String>> {
    let metadata: Option<TtlMetadata> = get_json(format!("{}_ttl", key))?;

    match metadata {
        Some(meta) => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| format!("Failed to get current time: {}", e))?
                .as_secs();

            if now < meta.expires_at {
                Ok(Some(meta.value))
            } else {
                // Expired, remove it
                remove(format!("{}_ttl", key))?;
                Ok(None)
            }
        }
        None => Ok(None),
    }
}
