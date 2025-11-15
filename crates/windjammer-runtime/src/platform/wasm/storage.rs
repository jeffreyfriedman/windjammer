/// WASM implementation of std::storage using localStorage and sessionStorage
#[cfg(target_arch = "wasm32")]
use web_sys::{window, Storage};

pub type StorageResult<T> = Result<T, String>;

/// Storage backend types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageBackend {
    Local,      // localStorage
    Session,    // sessionStorage
    Persistent, // IndexedDB (not yet implemented, falls back to localStorage)
}

/// Get the storage object for a given backend
#[cfg(target_arch = "wasm32")]
fn get_storage(backend: StorageBackend) -> StorageResult<Storage> {
    let window = window().ok_or_else(|| "No window object".to_string())?;

    match backend {
        StorageBackend::Local | StorageBackend::Persistent => window
            .local_storage()
            .map_err(|e| format!("Failed to get localStorage: {:?}", e))?
            .ok_or_else(|| "localStorage not available".to_string()),
        StorageBackend::Session => window
            .session_storage()
            .map_err(|e| format!("Failed to get sessionStorage: {:?}", e))?
            .ok_or_else(|| "sessionStorage not available".to_string()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn get_storage(_backend: StorageBackend) -> StorageResult<()> {
    Err("Storage only available in WASM".to_string())
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
#[cfg(target_arch = "wasm32")]
pub fn set_with_backend(key: String, value: String, backend: StorageBackend) -> StorageResult<()> {
    let storage = get_storage(backend)?;
    storage
        .set_item(&key, &value)
        .map_err(|e| format!("Failed to set storage item: {:?}", e))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn set_with_backend(
    _key: String,
    _value: String,
    _backend: StorageBackend,
) -> StorageResult<()> {
    Err("Storage only available in WASM".to_string())
}

/// Get with specific backend
#[cfg(target_arch = "wasm32")]
pub fn get_with_backend(key: String, backend: StorageBackend) -> StorageResult<Option<String>> {
    let storage = get_storage(backend)?;
    storage
        .get_item(&key)
        .map_err(|e| format!("Failed to get storage item: {:?}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_with_backend(_key: String, _backend: StorageBackend) -> StorageResult<Option<String>> {
    Err("Storage only available in WASM".to_string())
}

/// Remove with specific backend
#[cfg(target_arch = "wasm32")]
pub fn remove_with_backend(key: String, backend: StorageBackend) -> StorageResult<()> {
    let storage = get_storage(backend)?;
    storage
        .remove_item(&key)
        .map_err(|e| format!("Failed to remove storage item: {:?}", e))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn remove_with_backend(_key: String, _backend: StorageBackend) -> StorageResult<()> {
    Err("Storage only available in WASM".to_string())
}

/// Clear with specific backend
#[cfg(target_arch = "wasm32")]
pub fn clear_with_backend(backend: StorageBackend) -> StorageResult<()> {
    let storage = get_storage(backend)?;
    storage
        .clear()
        .map_err(|e| format!("Failed to clear storage: {:?}", e))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn clear_with_backend(_backend: StorageBackend) -> StorageResult<()> {
    Err("Storage only available in WASM".to_string())
}

/// Keys with specific backend
#[cfg(target_arch = "wasm32")]
pub fn keys_with_backend(backend: StorageBackend) -> StorageResult<Vec<String>> {
    let storage = get_storage(backend)?;
    let length = storage
        .length()
        .map_err(|e| format!("Failed to get storage length: {:?}", e))?;

    let mut keys = Vec::new();
    for i in 0..length {
        if let Ok(Some(key)) = storage.key(i) {
            keys.push(key);
        }
    }

    Ok(keys)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn keys_with_backend(_backend: StorageBackend) -> StorageResult<Vec<String>> {
    Err("Storage only available in WASM".to_string())
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

// Binary storage (base64 encoded in localStorage)

/// Store binary data
pub fn set_bytes(key: String, data: Vec<u8>) -> StorageResult<()> {
    let base64 = base64::encode(&data);
    set(key, base64)
}

/// Get binary data
pub fn get_bytes(key: String) -> StorageResult<Option<Vec<u8>>> {
    match get(key)? {
        Some(base64) => {
            let data =
                base64::decode(&base64).map_err(|e| format!("Failed to decode base64: {}", e))?;
            Ok(Some(data))
        }
        None => Ok(None),
    }
}

// Cache management with TTL

/// Metadata for TTL storage
#[derive(serde::Serialize, serde::Deserialize)]
struct TtlMetadata {
    value: String,
    expires_at: f64, // JavaScript timestamp (milliseconds since epoch)
}

/// Set with expiration (seconds)
#[cfg(target_arch = "wasm32")]
pub fn set_with_ttl(key: String, value: String, ttl: i32) -> StorageResult<()> {
    use js_sys::Date;

    let now = Date::now(); // milliseconds
    let expires_at = now + (ttl as f64 * 1000.0);

    let metadata = TtlMetadata { value, expires_at };

    set_json(format!("{}_ttl", key), metadata)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn set_with_ttl(_key: String, _value: String, _ttl: i32) -> StorageResult<()> {
    Err("Storage only available in WASM".to_string())
}

/// Get with automatic expiration check
#[cfg(target_arch = "wasm32")]
pub fn get_with_ttl(key: String) -> StorageResult<Option<String>> {
    use js_sys::Date;

    let metadata: Option<TtlMetadata> = get_json(format!("{}_ttl", key))?;

    match metadata {
        Some(meta) => {
            let now = Date::now();

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

#[cfg(not(target_arch = "wasm32"))]
pub fn get_with_ttl(_key: String) -> StorageResult<Option<String>> {
    Err("Storage only available in WASM".to_string())
}
