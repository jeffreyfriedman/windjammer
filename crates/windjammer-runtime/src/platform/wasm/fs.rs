use js_sys::{Array, Function, Reflect};
/// WASM implementation of std::fs
/// Automatically uses Tauri when available (desktop), falls back to localStorage (browser)
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use web_sys::window;

pub type FsResult<T> = Result<T, String>;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
}

/// Check if Tauri is available
fn is_tauri() -> bool {
    window()
        .and_then(|w| Reflect::get(&w, &JsValue::from_str("__TAURI__")).ok())
        .map(|v| !v.is_undefined())
        .unwrap_or(false)
}

/// Call Tauri invoke command
async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("No window"))?;
    let tauri = Reflect::get(&window, &JsValue::from_str("__TAURI__"))?;
    let core = Reflect::get(&tauri, &JsValue::from_str("core"))?;
    let invoke_fn = Reflect::get(&core, &JsValue::from_str("invoke"))?;
    let invoke_fn = invoke_fn
        .dyn_into::<Function>()
        .map_err(|_| JsValue::from_str("invoke is not a function"))?;

    let promise = invoke_fn.call2(&JsValue::NULL, &JsValue::from_str(cmd), &args)?;
    let promise = promise
        .dyn_into::<js_sys::Promise>()
        .map_err(|_| JsValue::from_str("invoke did not return a promise"))?;

    JsFuture::from(promise).await
}

/// Read a file (Tauri or localStorage)
pub fn read_file(path: String) -> FsResult<String> {
    // For synchronous API, we use localStorage
    // Tauri version would need async, which we'll add later
    if is_tauri() {
        // For now, log that we detected Tauri but fall back to localStorage
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Tauri detected, but using localStorage for: {}",
            path
        )));
    }

    let storage = window()
        .ok_or("No window available")?
        .local_storage()
        .map_err(|e| format!("Failed to access localStorage: {:?}", e))?
        .ok_or("localStorage not available")?;

    let key = format!("wj_file:{}", path);
    storage
        .get_item(&key)
        .map_err(|e| format!("Failed to read file: {:?}", e))?
        .ok_or_else(|| format!("File not found: {}", path))
}

/// Read a file as bytes (for binary files)
pub fn read_bytes(path: String) -> FsResult<Vec<u8>> {
    // For now, read as string and convert to bytes
    // TODO: Implement proper binary storage using IndexedDB
    let content = read_file(path)?;
    Ok(content.into_bytes())
}

/// Write a file (Tauri or localStorage)
pub fn write_file(path: String, content: String) -> FsResult<()> {
    if is_tauri() {
        // Use Tauri command
        write_file_tauri(path, content)
    } else {
        // Use localStorage
        write_file_localstorage(path, content)
    }
}

/// Write file using Tauri (synchronous wrapper around async call)
fn write_file_tauri(path: String, content: String) -> FsResult<()> {
    // Call the Tauri bridge JavaScript function
    let window = window().ok_or("No window available")?;
    let tauri_bridge = Reflect::get(&window, &JsValue::from_str("__WINDJAMMER_TAURI__"))
        .map_err(|_| "Tauri bridge not found")?;
    let write_fn = Reflect::get(&tauri_bridge, &JsValue::from_str("writeFileSync"))
        .map_err(|_| "writeFileSync function not found")?;
    let write_fn = write_fn
        .dyn_into::<Function>()
        .map_err(|_| "writeFileSync is not a function")?;

    // Call the synchronous wrapper
    write_fn
        .call2(
            &JsValue::NULL,
            &JsValue::from_str(&path),
            &JsValue::from_str(&content),
        )
        .map_err(|e| format!("Failed to call writeFileSync: {:?}", e))?;

    Ok(())
}

/// Write file to localStorage
fn write_file_localstorage(path: String, content: String) -> FsResult<()> {
    let storage = window()
        .ok_or("No window available")?
        .local_storage()
        .map_err(|e| format!("Failed to access localStorage: {:?}", e))?
        .ok_or("localStorage not available")?;

    let key = format!("wj_file:{}", path);
    storage
        .set_item(&key, &content)
        .map_err(|e| format!("Failed to write file: {:?}", e))?;

    // Also track this file in the directory listing
    let dir = get_directory_from_path(&path);
    add_file_to_directory(&dir, &path)?;

    Ok(())
}

/// List files in a directory
pub fn list_directory(path: String) -> FsResult<Vec<FileEntry>> {
    let storage = window()
        .ok_or("No window available")?
        .local_storage()
        .map_err(|e| format!("Failed to access localStorage: {:?}", e))?
        .ok_or("localStorage not available")?;

    let dir_key = format!("wj_dir:{}", path);
    let files_json = storage
        .get_item(&dir_key)
        .map_err(|e| format!("Failed to list directory: {:?}", e))?
        .unwrap_or_else(|| "[]".to_string());

    // Parse JSON array of file paths
    let files: Vec<String> = serde_json::from_str(&files_json).unwrap_or_else(|_| Vec::new());

    let mut entries = Vec::new();
    for file_path in files {
        let name = file_path
            .split('/')
            .last()
            .unwrap_or(&file_path)
            .to_string();
        entries.push(FileEntry {
            name,
            path: file_path.clone(),
            is_directory: false,
        });
    }

    Ok(entries)
}

/// Create a directory (Tauri or localStorage)
pub fn create_directory(path: String) -> FsResult<()> {
    if is_tauri() {
        create_directory_tauri(path)
    } else {
        create_directory_localstorage(path)
    }
}

/// Create directory using Tauri
fn create_directory_tauri(path: String) -> FsResult<()> {
    let window = window().ok_or("No window available")?;
    let tauri_bridge = Reflect::get(&window, &JsValue::from_str("__WINDJAMMER_TAURI__"))
        .map_err(|_| "Tauri bridge not found")?;
    let create_fn = Reflect::get(&tauri_bridge, &JsValue::from_str("createDirectorySync"))
        .map_err(|_| "createDirectorySync function not found")?;
    let create_fn = create_fn
        .dyn_into::<Function>()
        .map_err(|_| "createDirectorySync is not a function")?;

    create_fn
        .call1(&JsValue::NULL, &JsValue::from_str(&path))
        .map_err(|e| format!("Failed to call createDirectorySync: {:?}", e))?;

    Ok(())
}

/// Create directory in localStorage
fn create_directory_localstorage(path: String) -> FsResult<()> {
    let storage = window()
        .ok_or("No window available")?
        .local_storage()
        .map_err(|e| format!("Failed to access localStorage: {:?}", e))?
        .ok_or("localStorage not available")?;

    let dir_key = format!("wj_dir:{}", path);
    // Initialize empty directory
    storage
        .set_item(&dir_key, "[]")
        .map_err(|e| format!("Failed to create directory: {:?}", e))?;

    Ok(())
}

/// Delete a file from localStorage
pub fn delete_file(path: String) -> FsResult<()> {
    let storage = window()
        .ok_or("No window available")?
        .local_storage()
        .map_err(|e| format!("Failed to access localStorage: {:?}", e))?
        .ok_or("localStorage not available")?;

    let key = format!("wj_file:{}", path);
    storage
        .remove_item(&key)
        .map_err(|e| format!("Failed to delete file: {:?}", e))?;

    // Remove from directory listing
    let dir = get_directory_from_path(&path);
    remove_file_from_directory(&dir, &path)?;

    Ok(())
}

/// Check if a file exists
pub fn file_exists(path: String) -> bool {
    window()
        .and_then(|w| w.local_storage().ok())
        .and_then(|s| s)
        .and_then(|storage| {
            let key = format!("wj_file:{}", path);
            storage.get_item(&key).ok().flatten()
        })
        .is_some()
}

// Helper functions

fn get_directory_from_path(path: &str) -> String {
    let parts: Vec<&str> = path.rsplitn(2, '/').collect();
    if parts.len() > 1 {
        parts[1].to_string()
    } else {
        "/".to_string()
    }
}

fn add_file_to_directory(dir: &str, file_path: &str) -> FsResult<()> {
    let storage = window()
        .ok_or("No window available")?
        .local_storage()
        .map_err(|e| format!("Failed to access localStorage: {:?}", e))?
        .ok_or("localStorage not available")?;

    let dir_key = format!("wj_dir:{}", dir);
    let files_json = storage
        .get_item(&dir_key)
        .map_err(|e| format!("Failed to read directory: {:?}", e))?
        .unwrap_or_else(|| "[]".to_string());

    let mut files: Vec<String> = serde_json::from_str(&files_json).unwrap_or_else(|_| Vec::new());

    if !files.contains(&file_path.to_string()) {
        files.push(file_path.to_string());
        let updated_json = serde_json::to_string(&files)
            .map_err(|e| format!("Failed to serialize directory: {}", e))?;

        storage
            .set_item(&dir_key, &updated_json)
            .map_err(|e| format!("Failed to update directory: {:?}", e))?;
    }

    Ok(())
}

fn remove_file_from_directory(dir: &str, file_path: &str) -> FsResult<()> {
    let storage = window()
        .ok_or("No window available")?
        .local_storage()
        .map_err(|e| format!("Failed to access localStorage: {:?}", e))?
        .ok_or("localStorage not available")?;

    let dir_key = format!("wj_dir:{}", dir);
    let files_json = storage
        .get_item(&dir_key)
        .map_err(|e| format!("Failed to read directory: {:?}", e))?
        .unwrap_or_else(|| "[]".to_string());

    let mut files: Vec<String> = serde_json::from_str(&files_json).unwrap_or_else(|_| Vec::new());

    files.retain(|f| f != file_path);

    let updated_json = serde_json::to_string(&files)
        .map_err(|e| format!("Failed to serialize directory: {}", e))?;

    storage
        .set_item(&dir_key, &updated_json)
        .map_err(|e| format!("Failed to update directory: {:?}", e))?;

    Ok(())
}
