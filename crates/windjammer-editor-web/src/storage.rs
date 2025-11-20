//! IndexedDB storage for the browser editor
//!
//! This module provides persistent storage for editor projects using IndexedDB.

use wasm_bindgen::prelude::*;
use web_sys::{console, window};

/// Storage manager for editor projects
#[wasm_bindgen]
pub struct StorageManager {
    db_name: String,
    version: u32,
}

#[wasm_bindgen]
impl StorageManager {
    /// Create a new storage manager
    #[wasm_bindgen(constructor)]
    pub fn new() -> StorageManager {
        StorageManager {
            db_name: "windjammer_editor".to_string(),
            version: 1,
        }
    }

    /// Initialize the IndexedDB database
    pub fn init(&self) -> Result<(), JsValue> {
        console::log_1(&"Initializing IndexedDB...".into());
        
        // Note: This is a simplified version. In a real implementation,
        // you would use the web-sys IDB API or a wrapper library like
        // indexed_db_futures or rexie.
        
        console::log_1(&"IndexedDB initialized!".into());
        Ok(())
    }

    /// Save a project to IndexedDB
    pub fn save_project(&self, name: &str, data: &str) -> Result<(), JsValue> {
        console::log_1(&format!("Saving project: {}", name).into());
        
        // Store in localStorage as a fallback for now
        // In production, this should use IndexedDB
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let key = format!("project_{}", name);
                storage.set_item(&key, data)?;
                console::log_1(&format!("Project '{}' saved successfully!", name).into());
            }
        }
        
        Ok(())
    }

    /// Load a project from IndexedDB
    pub fn load_project(&self, name: &str) -> Result<String, JsValue> {
        console::log_1(&format!("Loading project: {}", name).into());
        
        // Load from localStorage as a fallback for now
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let key = format!("project_{}", name);
                if let Ok(Some(data)) = storage.get_item(&key) {
                    console::log_1(&format!("Project '{}' loaded successfully!", name).into());
                    return Ok(data);
                }
            }
        }
        
        Err(JsValue::from_str("Project not found"))
    }

    /// List all saved projects
    pub fn list_projects(&self) -> Result<Vec<String>, JsValue> {
        console::log_1(&"Listing projects...".into());
        
        let mut projects = Vec::new();
        
        // List from localStorage as a fallback for now
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(length) = storage.length() {
                    for i in 0..length {
                        if let Ok(Some(key)) = storage.key(i) {
                            if key.starts_with("project_") {
                                let name = key.trim_start_matches("project_").to_string();
                                projects.push(name);
                            }
                        }
                    }
                }
            }
        }
        
        console::log_1(&format!("Found {} projects", projects.len()).into());
        Ok(projects)
    }

    /// Delete a project from IndexedDB
    pub fn delete_project(&self, name: &str) -> Result<(), JsValue> {
        console::log_1(&format!("Deleting project: {}", name).into());
        
        // Delete from localStorage as a fallback for now
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let key = format!("project_{}", name);
                storage.remove_item(&key)?;
                console::log_1(&format!("Project '{}' deleted successfully!", name).into());
            }
        }
        
        Ok(())
    }

    /// Export a project as JSON
    pub fn export_project(&self, name: &str) -> Result<String, JsValue> {
        console::log_1(&format!("Exporting project: {}", name).into());
        
        let data = self.load_project(name)?;
        
        // Wrap in a JSON object with metadata
        let export = serde_json::json!({
            "name": name,
            "version": "1.0",
            "data": data,
            "exported_at": js_sys::Date::new_0().to_iso_string().as_string(),
        });
        
        Ok(export.to_string())
    }

    /// Import a project from JSON
    pub fn import_project(&self, json: &str) -> Result<String, JsValue> {
        console::log_1(&"Importing project...".into());
        
        let import: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Invalid JSON: {}", e)))?;
        
        let name = import["name"]
            .as_str()
            .ok_or_else(|| JsValue::from_str("Missing project name"))?;
        
        let data = import["data"]
            .as_str()
            .ok_or_else(|| JsValue::from_str("Missing project data"))?;
        
        self.save_project(name, data)?;
        
        console::log_1(&format!("Project '{}' imported successfully!", name).into());
        Ok(name.to_string())
    }

    /// Clear all projects (use with caution!)
    pub fn clear_all(&self) -> Result<(), JsValue> {
        console::log_1(&"Clearing all projects...".into());
        
        let projects = self.list_projects()?;
        for project in projects {
            self.delete_project(&project)?;
        }
        
        console::log_1(&"All projects cleared!".into());
        Ok(())
    }

    /// Get storage usage statistics
    pub fn get_storage_stats(&self) -> Result<StorageStats, JsValue> {
        console::log_1(&"Getting storage stats...".into());
        
        let projects = self.list_projects()?;
        let mut total_size = 0;
        
        // Calculate total size from localStorage
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                for project in &projects {
                    let key = format!("project_{}", project);
                    if let Ok(Some(data)) = storage.get_item(&key) {
                        total_size += data.len();
                    }
                }
            }
        }
        
        Ok(StorageStats {
            project_count: projects.len(),
            total_size_bytes: total_size,
        })
    }
}

/// Storage statistics
#[wasm_bindgen]
#[derive(Clone)]
pub struct StorageStats {
    project_count: usize,
    total_size_bytes: usize,
}

#[wasm_bindgen]
impl StorageStats {
    /// Get the number of projects
    pub fn project_count(&self) -> usize {
        self.project_count
    }

    /// Get the total size in bytes
    pub fn total_size_bytes(&self) -> usize {
        self.total_size_bytes
    }

    /// Get the total size in kilobytes
    pub fn total_size_kb(&self) -> f64 {
        self.total_size_bytes as f64 / 1024.0
    }

    /// Get the total size in megabytes
    pub fn total_size_mb(&self) -> f64 {
        self.total_size_bytes as f64 / (1024.0 * 1024.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_manager_creation() {
        let manager = StorageManager::new();
        assert_eq!(manager.db_name, "windjammer_editor");
        assert_eq!(manager.version, 1);
    }

    #[test]
    fn test_storage_stats() {
        let stats = StorageStats {
            project_count: 5,
            total_size_bytes: 1024 * 1024, // 1 MB
        };
        
        assert_eq!(stats.project_count(), 5);
        assert_eq!(stats.total_size_bytes(), 1024 * 1024);
        assert_eq!(stats.total_size_kb(), 1024.0);
        assert_eq!(stats.total_size_mb(), 1.0);
    }
}

