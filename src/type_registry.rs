//! TypeRegistry - Maps type names to their defining modules
//!
//! This module provides a centralized registry that tracks where each type is defined.
//! This enables the code generator to produce correct import paths without guessing.
//!
//! Example:
//! - Type "Vec2" is defined in "math/vec2.wj" → module path "math::vec2"
//! - Type "Camera2D" is defined in "rendering/camera2d.wj" → module path "rendering::camera2d"

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::parser::ast::Item;

/// Maps type names and function names to their defining module paths
#[derive(Debug, Clone, Default)]
pub struct TypeRegistry {
    /// Map: TypeName -> ModulePath
    /// Example: "Vec2" -> "math::vec2"
    types: HashMap<String, String>,
    
    /// Map: FunctionName -> ModulePath
    /// Example: "check_validity" -> "validator"
    functions: HashMap<String, String>,
    
    /// Map: TypeName -> FilePath (for debugging)
    /// Example: "Vec2" -> "src_wj/math/vec2.wj"
    file_paths: HashMap<String, PathBuf>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
            functions: HashMap::new(),
            file_paths: HashMap::new(),
        }
    }
    
    /// Register a type with its module path
    pub fn register_type(&mut self, type_name: String, module_path: String, file_path: PathBuf) {
        self.types.insert(type_name.clone(), module_path);
        self.file_paths.insert(type_name, file_path);
    }
    
    /// Register a function with its module path
    pub fn register_function(&mut self, function_name: String, module_path: String) {
        self.functions.insert(function_name, module_path);
    }
    
    /// Look up the module path for a type
    /// Returns None if the type is not in the registry
    pub fn lookup_type(&self, type_name: &str) -> Option<&str> {
        self.types.get(type_name).map(|s| s.as_str())
    }
    
    /// Look up the module path for a function
    /// Returns None if the function is not in the registry
    pub fn lookup_function(&self, function_name: &str) -> Option<&str> {
        self.functions.get(function_name).map(|s| s.as_str())
    }
    
    /// Look up either type or function (tries type first, then function)
    pub fn lookup(&self, name: &str) -> Option<&str> {
        self.lookup_type(name).or_else(|| self.lookup_function(name))
    }
    
    /// Get the file path where a type is defined (for debugging)
    pub fn get_file_path(&self, type_name: &str) -> Option<&Path> {
        self.file_paths.get(type_name).map(|p| p.as_path())
    }
    
    /// Scan a single .wj file and register all types defined in it
    pub fn scan_file(&mut self, file_path: &Path, ast: &[Item]) -> Result<(), String> {
        // Compute module path from file path
        // Example: "src_wj/math/vec2.wj" -> "math::vec2"
        let module_path = Self::file_path_to_module_path(file_path)?;
        
        // Extract all type and function definitions from the AST
        for item in ast {
            match item {
                Item::Struct { decl, .. } => {
                    self.register_type(decl.name.clone(), module_path.clone(), file_path.to_path_buf());
                }
                Item::Enum { decl, .. } => {
                    self.register_type(decl.name.clone(), module_path.clone(), file_path.to_path_buf());
                }
                Item::Trait { decl, .. } => {
                    self.register_type(decl.name.clone(), module_path.clone(), file_path.to_path_buf());
                }
                Item::Function { decl, .. } => {
                    // Register top-level functions
                    if decl.is_pub {
                        self.register_function(decl.name.clone(), module_path.clone());
                    }
                }
                Item::TypeAlias { name, is_pub, .. } => {
                    if *is_pub {
                        self.register_type(name.clone(), module_path.clone(), file_path.to_path_buf());
                    }
                }
                Item::Mod { items, .. } => {
                    // For inline modules, types and functions are defined in the parent module
                    // Example: mod utils { struct Helper {} } -> Helper is in current module
                    for nested_item in items {
                        match nested_item {
                            Item::Struct { decl, .. } => {
                                self.register_type(decl.name.clone(), module_path.clone(), file_path.to_path_buf());
                            }
                            Item::Enum { decl, .. } => {
                                self.register_type(decl.name.clone(), module_path.clone(), file_path.to_path_buf());
                            }
                            Item::Trait { decl, .. } => {
                                self.register_type(decl.name.clone(), module_path.clone(), file_path.to_path_buf());
                            }
                            Item::Function { decl, .. } => {
                                if decl.is_pub {
                                    self.register_function(decl.name.clone(), module_path.clone());
                                }
                            }
                            Item::TypeAlias { name, is_pub, .. } => {
                                if *is_pub {
                                    self.register_type(name.clone(), module_path.clone(), file_path.to_path_buf());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    /// Convert a file path to a module path
    /// Since all generated files are FLAT in src/generated/, we only use the filename
    /// Example: "src_wj/math/vec2.wj" -> "vec2"
    /// Example: "src_wj/utils/validator.wj" -> "validator"
    fn file_path_to_module_path(file_path: &Path) -> Result<String, String> {
        // Extract just the filename without extension
        let filename = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("Invalid file path: {:?}", file_path))?;
        
        Ok(filename.to_string())
    }
    
    /// Generate a correct use statement for a type
    /// Example: "Vec2" in context "rendering" -> "use super::vec2::Vec2"
    pub fn generate_use_statement(&self, type_name: &str, current_module: &str) -> Option<String> {
        let module_path = self.lookup(type_name)?;
        
        // If the type is in the same module, no import needed
        if module_path == current_module {
            return None;
        }
        
        // Generate relative import using super::
        // All generated files are in src/generated/, so we use super:: to reference siblings
        let use_path = if module_path.contains("::") {
            // Nested module: math::vec2 -> super::math::vec2
            format!("use super::{}::{};", module_path, type_name)
        } else {
            // Top-level module: vec2 -> super::vec2
            format!("use super::{}::{};", module_path, type_name)
        };
        
        Some(use_path)
    }
    
    /// Get all registered types (for debugging)
    pub fn all_types(&self) -> Vec<&str> {
        self.types.keys().map(|s| s.as_str()).collect()
    }
    
    /// Get all registered functions (for debugging)
    pub fn all_functions(&self) -> Vec<&str> {
        self.functions.keys().map(|s| s.as_str()).collect()
    }
    
    /// Get total count of registered types and functions
    pub fn total_count(&self) -> usize {
        self.types.len() + self.functions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Item, StructDecl, TypeParam};
    
    #[test]
    fn test_file_path_to_module_path() {
        // Since all generated files are flat, only the filename matters
        assert_eq!(
            TypeRegistry::file_path_to_module_path(Path::new("src_wj/math/vec2.wj")).unwrap(),
            "vec2"
        );
        
        assert_eq!(
            TypeRegistry::file_path_to_module_path(Path::new("math/vec2.wj")).unwrap(),
            "vec2"
        );
        
        assert_eq!(
            TypeRegistry::file_path_to_module_path(Path::new("src_wj/rendering/camera2d.wj")).unwrap(),
            "camera2d"
        );
    }
    
    #[test]
    fn test_register_and_lookup() {
        let mut registry = TypeRegistry::new();
        
        registry.register_type(
            "Vec2".to_string(),
            "vec2".to_string(),
            PathBuf::from("src_wj/math/vec2.wj")
        );
        
        registry.register_function(
            "check_validity".to_string(),
            "validator".to_string()
        );
        
        assert_eq!(registry.lookup_type("Vec2"), Some("vec2"));
        assert_eq!(registry.lookup_type("Vec3"), None);
        assert_eq!(registry.lookup_function("check_validity"), Some("validator"));
        assert_eq!(registry.lookup("Vec2"), Some("vec2"));
        assert_eq!(registry.lookup("check_validity"), Some("validator"));
    }
    
    #[test]
    fn test_generate_use_statement() {
        let mut registry = TypeRegistry::new();
        
        registry.register_type(
            "Vec2".to_string(),
            "vec2".to_string(),
            PathBuf::from("src_wj/math/vec2.wj")
        );
        
        registry.register_type(
            "Camera2D".to_string(),
            "camera2d".to_string(),
            PathBuf::from("src_wj/rendering/camera2d.wj")
        );
        
        // Use Vec2 from camera2d module -> generates import
        assert_eq!(
            registry.generate_use_statement("Vec2", "camera2d"),
            Some("use super::vec2::Vec2;".to_string())
        );
        
        // Use Camera2D in camera2d module -> no import needed
        assert_eq!(
            registry.generate_use_statement("Camera2D", "camera2d"),
            None
        );
    }
}
