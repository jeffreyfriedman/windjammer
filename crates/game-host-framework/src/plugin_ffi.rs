// Windjammer Plugin System - C FFI Layer
// Enables dynamic plugin loading from C/C++/other languages

use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr;
use std::sync::{Arc, Mutex};
use crate::plugin::{App, Plugin, PluginCategory, PluginDependency, PluginError, VersionReq};

// ============================================================================
// C-Compatible Types
// ============================================================================

/// Opaque handle to the Windjammer App
#[repr(C)]
pub struct WjApp {
    _private: [u8; 0],
}

/// Plugin error codes for FFI
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WjPluginErrorCode {
    Ok = 0,
    InvalidParameter = 1,
    PluginNotFound = 2,
    DependencyError = 3,
    AlreadyLoaded = 4,
    LoadFailed = 5,
    UnloadFailed = 6,
    VersionMismatch = 7,
    CircularDependency = 8,
}

/// Plugin category enum for FFI
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WjPluginCategory {
    Rendering = 0,
    Physics = 1,
    Audio = 2,
    AI = 3,
    Editor = 4,
    Assets = 5,
    Networking = 6,
    Platform = 7,
    Other = 8,
}

impl From<WjPluginCategory> for PluginCategory {
    fn from(cat: WjPluginCategory) -> Self {
        match cat {
            WjPluginCategory::Rendering => PluginCategory::Rendering,
            WjPluginCategory::Physics => PluginCategory::Physics,
            WjPluginCategory::Audio => PluginCategory::Audio,
            WjPluginCategory::AI => PluginCategory::AI,
            WjPluginCategory::Editor => PluginCategory::Editor,
            WjPluginCategory::Assets => PluginCategory::Assets,
            WjPluginCategory::Networking => PluginCategory::Networking,
            WjPluginCategory::Platform => PluginCategory::Platform,
            WjPluginCategory::Other => PluginCategory::Other,
        }
    }
}

/// Plugin dependency for FFI
#[repr(C)]
pub struct WjPluginDependency {
    pub name: *const c_char,
    pub version: *const c_char,
}

/// Plugin metadata for FFI
#[repr(C)]
pub struct WjPluginInfo {
    pub name: *const c_char,
    pub version: *const c_char,
    pub description: *const c_char,
    pub author: *const c_char,
    pub license: *const c_char,
    pub category: WjPluginCategory,
    pub supports_hot_reload: bool,
}

// ============================================================================
// Plugin Function Pointers
// ============================================================================

/// Plugin initialization function
pub type WjPluginInitFn = extern "C" fn(app: *mut WjApp) -> WjPluginErrorCode;

/// Plugin cleanup function
pub type WjPluginCleanupFn = extern "C" fn(app: *mut WjApp) -> WjPluginErrorCode;

/// Plugin info function
pub type WjPluginInfoFn = extern "C" fn() -> WjPluginInfo;

/// Plugin dependencies function
pub type WjPluginDependenciesFn = extern "C" fn(out_count: *mut usize) -> *const WjPluginDependency;

// ============================================================================
// Dynamic Plugin Wrapper
// ============================================================================

/// Wrapper for dynamically loaded plugins
pub struct DynamicPlugin {
    name: String,
    version: String,
    description: String,
    author: String,
    license: String,
    category: PluginCategory,
    supports_hot_reload: bool,
    dependencies: Vec<PluginDependency>,
    
    // Function pointers
    init_fn: WjPluginInitFn,
    cleanup_fn: WjPluginCleanupFn,
    
    // Library handle (for hot-reload)
    #[cfg(feature = "dynamic_plugins")]
    library: Arc<Mutex<libloading::Library>>,
}

impl DynamicPlugin {
    /// Load a dynamic plugin from a shared library
    #[cfg(feature = "dynamic_plugins")]
    pub fn load(path: &str) -> Result<Self, PluginError> {
        use libloading::{Library, Symbol};
        
        // Load the shared library
        let library = unsafe {
            Library::new(path).map_err(|e| {
                PluginError::LoadFailed(format!("Failed to load library: {}", e))
            })?
        };
        
        // Load plugin info
        let info_fn: Symbol<WjPluginInfoFn> = unsafe {
            library.get(b"wj_plugin_info").map_err(|e| {
                PluginError::LoadFailed(format!("Failed to load wj_plugin_info: {}", e))
            })?
        };
        
        let info = info_fn();
        
        // Convert C strings to Rust strings
        let name = unsafe { CStr::from_ptr(info.name) }
            .to_str()
            .map_err(|_| PluginError::LoadFailed("Invalid plugin name".to_string()))?
            .to_string();
        
        let version = unsafe { CStr::from_ptr(info.version) }
            .to_str()
            .map_err(|_| PluginError::LoadFailed("Invalid plugin version".to_string()))?
            .to_string();
        
        let description = unsafe { CStr::from_ptr(info.description) }
            .to_str()
            .unwrap_or("")
            .to_string();
        
        let author = unsafe { CStr::from_ptr(info.author) }
            .to_str()
            .unwrap_or("")
            .to_string();
        
        let license = unsafe { CStr::from_ptr(info.license) }
            .to_str()
            .unwrap_or("")
            .to_string();
        
        // Load dependencies
        let mut dependencies = Vec::new();
        let deps_fn: Result<Symbol<WjPluginDependenciesFn>, _> = unsafe {
            library.get(b"wj_plugin_dependencies")
        };
        
        if let Ok(deps_fn) = deps_fn {
            let mut count = 0;
            let deps_ptr = deps_fn(&mut count as *mut usize);
            
            if !deps_ptr.is_null() && count > 0 {
                let deps_slice = unsafe { std::slice::from_raw_parts(deps_ptr, count) };
                
                for dep in deps_slice {
                    let dep_name = unsafe { CStr::from_ptr(dep.name) }
                        .to_str()
                        .map_err(|_| PluginError::LoadFailed("Invalid dependency name".to_string()))?
                        .to_string();
                    
                    let dep_version = unsafe { CStr::from_ptr(dep.version) }
                        .to_str()
                        .map_err(|_| PluginError::LoadFailed("Invalid dependency version".to_string()))?
                        .to_string();
                    
                    let version_req = VersionReq::parse(&dep_version).unwrap_or_else(|_| {
                        // Default to wildcard (any version)
                        VersionReq::parse("*").unwrap()
                    });
                    
                    dependencies.push(PluginDependency {
                        name: dep_name,
                        version: version_req,
                    });
                }
            }
        }
        
        // Load init function
        let init_fn: Symbol<WjPluginInitFn> = unsafe {
            library.get(b"wj_plugin_init").map_err(|e| {
                PluginError::LoadFailed(format!("Failed to load wj_plugin_init: {}", e))
            })?
        };
        
        // Load cleanup function
        let cleanup_fn: Symbol<WjPluginCleanupFn> = unsafe {
            library.get(b"wj_plugin_cleanup").map_err(|e| {
                PluginError::LoadFailed(format!("Failed to load wj_plugin_cleanup: {}", e))
            })?
        };
        
        // Store function pointers (need to transmute to remove lifetime)
        let init_fn_ptr: WjPluginInitFn = unsafe { std::mem::transmute(*init_fn) };
        let cleanup_fn_ptr: WjPluginCleanupFn = unsafe { std::mem::transmute(*cleanup_fn) };
        
        Ok(Self {
            name,
            version,
            description,
            author,
            license,
            category: info.category.into(),
            supports_hot_reload: info.supports_hot_reload,
            dependencies,
            init_fn: init_fn_ptr,
            cleanup_fn: cleanup_fn_ptr,
            library: Arc::new(Mutex::new(library)),
        })
    }
    
    /// Reload the plugin (hot-reload)
    #[cfg(feature = "dynamic_plugins")]
    pub fn reload(&mut self, path: &str) -> Result<(), PluginError> {
        // Unload old library
        drop(self.library.lock().unwrap());
        
        // Load new library
        let new_plugin = Self::load(path)?;
        
        // Update function pointers
        self.init_fn = new_plugin.init_fn;
        self.cleanup_fn = new_plugin.cleanup_fn;
        self.library = new_plugin.library;
        
        Ok(())
    }
}

impl Plugin for DynamicPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        &self.version
    }
    
    fn dependencies(&self) -> Vec<PluginDependency> {
        self.dependencies.clone()
    }
    
    fn build(&self, app: &mut App) {
        // Call the plugin's init function
        let app_ptr = app as *mut App as *mut WjApp;
        let result = (self.init_fn)(app_ptr);
        
        if result != WjPluginErrorCode::Ok {
            eprintln!("Plugin {} initialization failed: {:?}", self.name, result);
        }
    }
    
    fn cleanup(&self, app: &mut App) {
        // Call the plugin's cleanup function
        let app_ptr = app as *mut App as *mut WjApp;
        let result = (self.cleanup_fn)(app_ptr);
        
        if result != WjPluginErrorCode::Ok {
            eprintln!("Plugin {} cleanup failed: {:?}", self.name, result);
        }
    }
    
    fn supports_hot_reload(&self) -> bool {
        self.supports_hot_reload
    }
    
    fn category(&self) -> PluginCategory {
        self.category
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    fn author(&self) -> &str {
        &self.author
    }
    
    fn license(&self) -> &str {
        &self.license
    }
}

// ============================================================================
// C API Functions (for plugin authors)
// ============================================================================

/// Helper macro for creating plugin info
#[macro_export]
macro_rules! wj_plugin_info {
    (
        name: $name:expr,
        version: $version:expr,
        description: $description:expr,
        author: $author:expr,
        license: $license:expr,
        category: $category:expr,
        hot_reload: $hot_reload:expr
    ) => {
        #[no_mangle]
        pub extern "C" fn wj_plugin_info() -> $crate::plugin_ffi::WjPluginInfo {
            use std::ffi::CString;
            
            static NAME: &str = concat!($name, "\0");
            static VERSION: &str = concat!($version, "\0");
            static DESCRIPTION: &str = concat!($description, "\0");
            static AUTHOR: &str = concat!($author, "\0");
            static LICENSE: &str = concat!($license, "\0");
            
            $crate::plugin_ffi::WjPluginInfo {
                name: NAME.as_ptr() as *const i8,
                version: VERSION.as_ptr() as *const i8,
                description: DESCRIPTION.as_ptr() as *const i8,
                author: AUTHOR.as_ptr() as *const i8,
                license: LICENSE.as_ptr() as *const i8,
                category: $category,
                supports_hot_reload: $hot_reload,
            }
        }
    };
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_code_conversion() {
        assert_eq!(WjPluginErrorCode::Ok as i32, 0);
        assert_eq!(WjPluginErrorCode::InvalidParameter as i32, 1);
    }
    
    #[test]
    fn test_category_conversion() {
        let cat = WjPluginCategory::Rendering;
        let rust_cat: PluginCategory = cat.into();
        assert_eq!(rust_cat, PluginCategory::Rendering);
    }
}

