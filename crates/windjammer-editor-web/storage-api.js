/**
 * Storage API for Windjammer Editor
 * 
 * This module provides a JavaScript API for persistent storage
 * using IndexedDB (via WASM bindings) and localStorage fallback.
 */

let storageManager = null;

/**
 * Initialize the storage manager
 * @param {Object} wasmModule - The WASM module
 */
export async function initStorage(wasmModule) {
    if (!wasmModule) {
        throw new Error('WASM module not provided');
    }
    
    try {
        console.log('🗄️ Initializing storage manager...');
        storageManager = new wasmModule.StorageManager();
        await storageManager.init();
        console.log('✅ Storage manager initialized!');
        return storageManager;
    } catch (error) {
        console.error('❌ Failed to initialize storage:', error);
        throw error;
    }
}

/**
 * Save a project
 * @param {string} name - Project name
 * @param {string} data - Project data (JSON string)
 */
export async function saveProject(name, data) {
    if (!storageManager) {
        throw new Error('Storage manager not initialized. Call initStorage() first.');
    }
    
    try {
        await storageManager.save_project(name, data);
        console.log(`💾 Project '${name}' saved successfully!`);
        return true;
    } catch (error) {
        console.error(`❌ Failed to save project '${name}':`, error);
        throw error;
    }
}

/**
 * Load a project
 * @param {string} name - Project name
 * @returns {Promise<string>} Project data (JSON string)
 */
export async function loadProject(name) {
    if (!storageManager) {
        throw new Error('Storage manager not initialized. Call initStorage() first.');
    }
    
    try {
        const data = await storageManager.load_project(name);
        console.log(`📂 Project '${name}' loaded successfully!`);
        return data;
    } catch (error) {
        console.error(`❌ Failed to load project '${name}':`, error);
        throw error;
    }
}

/**
 * List all saved projects
 * @returns {Promise<string[]>} Array of project names
 */
export async function listProjects() {
    if (!storageManager) {
        throw new Error('Storage manager not initialized. Call initStorage() first.');
    }
    
    try {
        const projects = await storageManager.list_projects();
        console.log(`📋 Found ${projects.length} projects`);
        return projects;
    } catch (error) {
        console.error('❌ Failed to list projects:', error);
        throw error;
    }
}

/**
 * Delete a project
 * @param {string} name - Project name
 */
export async function deleteProject(name) {
    if (!storageManager) {
        throw new Error('Storage manager not initialized. Call initStorage() first.');
    }
    
    try {
        await storageManager.delete_project(name);
        console.log(`🗑️ Project '${name}' deleted successfully!`);
        return true;
    } catch (error) {
        console.error(`❌ Failed to delete project '${name}':`, error);
        throw error;
    }
}

/**
 * Export a project as JSON
 * @param {string} name - Project name
 * @returns {Promise<string>} JSON string
 */
export async function exportProject(name) {
    if (!storageManager) {
        throw new Error('Storage manager not initialized. Call initStorage() first.');
    }
    
    try {
        const json = await storageManager.export_project(name);
        console.log(`📤 Project '${name}' exported successfully!`);
        return json;
    } catch (error) {
        console.error(`❌ Failed to export project '${name}':`, error);
        throw error;
    }
}

/**
 * Import a project from JSON
 * @param {string} json - JSON string
 * @returns {Promise<string>} Imported project name
 */
export async function importProject(json) {
    if (!storageManager) {
        throw new Error('Storage manager not initialized. Call initStorage() first.');
    }
    
    try {
        const name = await storageManager.import_project(json);
        console.log(`📥 Project '${name}' imported successfully!`);
        return name;
    } catch (error) {
        console.error('❌ Failed to import project:', error);
        throw error;
    }
}

/**
 * Download a project as a file
 * @param {string} name - Project name
 */
export async function downloadProject(name) {
    try {
        const json = await exportProject(name);
        
        // Create a blob and download link
        const blob = new Blob([json], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        
        const a = document.createElement('a');
        a.href = url;
        a.download = `${name}.windjammer.json`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        
        URL.revokeObjectURL(url);
        console.log(`⬇️ Project '${name}' downloaded successfully!`);
    } catch (error) {
        console.error(`❌ Failed to download project '${name}':`, error);
        throw error;
    }
}

/**
 * Upload and import a project file
 * @param {File} file - Project file
 * @returns {Promise<string>} Imported project name
 */
export async function uploadProject(file) {
    try {
        const text = await file.text();
        const name = await importProject(text);
        console.log(`⬆️ Project file uploaded and imported as '${name}'!`);
        return name;
    } catch (error) {
        console.error('❌ Failed to upload project:', error);
        throw error;
    }
}

/**
 * Get storage statistics
 * @returns {Promise<Object>} Storage stats
 */
export async function getStorageStats() {
    if (!storageManager) {
        throw new Error('Storage manager not initialized. Call initStorage() first.');
    }
    
    try {
        const stats = await storageManager.get_storage_stats();
        return {
            projectCount: stats.project_count(),
            totalSizeBytes: stats.total_size_bytes(),
            totalSizeKB: stats.total_size_kb(),
            totalSizeMB: stats.total_size_mb(),
        };
    } catch (error) {
        console.error('❌ Failed to get storage stats:', error);
        throw error;
    }
}

/**
 * Clear all projects (use with caution!)
 */
export async function clearAllProjects() {
    if (!storageManager) {
        throw new Error('Storage manager not initialized. Call initStorage() first.');
    }
    
    if (!confirm('Are you sure you want to delete ALL projects? This cannot be undone!')) {
        return false;
    }
    
    try {
        await storageManager.clear_all();
        console.log('🧹 All projects cleared!');
        return true;
    } catch (error) {
        console.error('❌ Failed to clear projects:', error);
        throw error;
    }
}

/**
 * Auto-save functionality
 */
let autoSaveInterval = null;
let autoSaveCallback = null;

/**
 * Enable auto-save
 * @param {Function} callback - Function that returns {name, data} to save
 * @param {number} intervalMs - Auto-save interval in milliseconds (default: 30000 = 30 seconds)
 */
export function enableAutoSave(callback, intervalMs = 30000) {
    if (autoSaveInterval) {
        clearInterval(autoSaveInterval);
    }
    
    autoSaveCallback = callback;
    
    autoSaveInterval = setInterval(async () => {
        try {
            const { name, data } = await callback();
            if (name && data) {
                await saveProject(name, data);
                console.log(`🔄 Auto-saved project '${name}'`);
            }
        } catch (error) {
            console.error('❌ Auto-save failed:', error);
        }
    }, intervalMs);
    
    console.log(`⏰ Auto-save enabled (every ${intervalMs / 1000} seconds)`);
}

/**
 * Disable auto-save
 */
export function disableAutoSave() {
    if (autoSaveInterval) {
        clearInterval(autoSaveInterval);
        autoSaveInterval = null;
        autoSaveCallback = null;
        console.log('⏸️ Auto-save disabled');
    }
}

/**
 * Get the storage manager instance
 */
export function getStorageManager() {
    return storageManager;
}

// Export all functions as a single object for convenience
export default {
    initStorage,
    saveProject,
    loadProject,
    listProjects,
    deleteProject,
    exportProject,
    importProject,
    downloadProject,
    uploadProject,
    getStorageStats,
    clearAllProjects,
    enableAutoSave,
    disableAutoSave,
    getStorageManager,
};

