// Windjammer Module System - The Windjammer Way!
//
// Philosophy:
// - Auto-discover modules from directory structure (compiler does the work)
// - Respect explicit pub mod / pub use declarations in mod.wj
// - Generate lib.rs/mod.rs automatically
// - No boilerplate - developer focuses on logic, not project structure
//
// This is NOT just copying Rust's module system!
// Windjammer infers structure while Rust forces manual declaration.

use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

/// Represents a discovered module in the project
#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub submodules: Vec<Module>,
    pub has_mod_wj: bool,
    pub is_public: bool,
}

/// Represents the complete module tree of a Windjammer project
#[derive(Debug)]
pub struct ModuleTree {
    pub root_path: PathBuf,
    pub root_modules: Vec<Module>,
}

impl ModuleTree {
    /// Check if a module exists at the given path
    pub fn has_module(&self, path: &[&str]) -> bool {
        if path.is_empty() {
            return false;
        }
        
        let mut current_modules = &self.root_modules;
        
        for (i, name) in path.iter().enumerate() {
            if let Some(module) = current_modules.iter().find(|m| m.name == *name) {
                if i == path.len() - 1 {
                    return true;
                }
                current_modules = &module.submodules;
            } else {
                return false;
            }
        }
        
        false
    }
}

/// Discover all modules in a Windjammer project (nested directories supported!)
///
/// The Windjammer Way:
/// - Auto-discover directories as modules (even without mod.wj)
/// - Respect mod.wj declarations if present
/// - Make the compiler smart, not the user
pub fn discover_nested_modules(root_path: &Path) -> Result<ModuleTree> {
    let root_modules = discover_modules_recursive(root_path, true)?;
    
    Ok(ModuleTree {
        root_path: root_path.to_path_buf(),
        root_modules,
    })
}

/// Recursively discover modules in a directory
fn discover_modules_recursive(dir_path: &Path, _is_root: bool) -> Result<Vec<Module>> {
    let mut modules = Vec::new();
    let mut subdirs = Vec::new();
    let mut wj_files = Vec::new();
    
    // Read directory contents
    for entry in fs::read_dir(dir_path).with_context(|| format!("Failed to read directory: {:?}", dir_path))? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        
        if path.is_dir() {
            // Skip common directories
            if name.starts_with('.') || name == "target" || name == "build" {
                continue;
            }
            subdirs.push((name.to_string(), path));
        } else if path.is_file() && name.ends_with(".wj") {
            wj_files.push((name.to_string(), path));
        }
    }
    
    // Process subdirectories as modules
    for (dir_name, dir_path) in subdirs {
        let has_mod_wj = dir_path.join("mod.wj").exists();
        let submodules = discover_modules_recursive(&dir_path, false)?;
        
        modules.push(Module {
            name: dir_name.clone(),
            path: dir_path,
            is_directory: true,
            submodules,
            has_mod_wj,
            is_public: true, // TODO: Parse mod.wj to determine visibility
        });
    }
    
    // Process .wj files as modules (excluding mod.wj)
    for (file_name, file_path) in wj_files {
        if file_name == "mod.wj" {
            continue; // mod.wj is not a module itself, it declares the parent module
        }
        
        let module_name = file_name.strip_suffix(".wj").unwrap().to_string();
        
        modules.push(Module {
            name: module_name,
            path: file_path,
            is_directory: false,
            submodules: Vec::new(),
            has_mod_wj: false,
            is_public: true, // TODO: Parse file to determine visibility
        });
    }
    
    // Sort modules alphabetically for consistent output
    modules.sort_by(|a, b| a.name.cmp(&b.name));
    
    Ok(modules)
}

/// Generate lib.rs content for a Windjammer project
///
/// The Windjammer Way:
/// - Respect explicit pub mod / pub use declarations from mod.wj
/// - Auto-generate pub mod for discovered directories
/// - Use wildcard re-exports ONLY if no explicit pub use exists
pub fn generate_lib_rs(module_tree: &ModuleTree) -> Result<String> {
    let mut content = String::from("// Auto-generated lib.rs by Windjammer\n");
    content.push_str("// This file declares all modules in your Windjammer project\n\n");
    
    // Check if mod.wj exists in root
    let root_mod_path = module_tree.root_path.join("mod.wj");
    let has_root_mod = root_mod_path.exists();
    
    if has_root_mod {
        // Parse mod.wj to extract pub mod and pub use declarations
        let mod_content = fs::read_to_string(&root_mod_path)?;
        let (pub_mods, pub_uses) = parse_mod_declarations(&mod_content);
        
        // Generate pub mod declarations
        content.push_str("// Module declarations\n");
        for module in &module_tree.root_modules {
            // Only generate pub mod if it's in the explicit list OR if no explicit list exists
            if pub_mods.is_empty() || pub_mods.contains(&module.name) {
                if module.is_public {
                    content.push_str(&format!("pub mod {};\n", module.name));
                } else {
                    content.push_str(&format!("mod {};\n", module.name));
                }
            }
        }
        
        // Generate re-exports
        if !pub_uses.is_empty() {
            content.push_str("\n// Re-exports (from mod.wj)\n");
            for pub_use in pub_uses {
                content.push_str(&format!("pub use {};\n", pub_use));
            }
        } else {
            // No explicit pub use - use wildcard re-exports
            content.push_str("\n// Auto-generated re-exports\n");
            for module in &module_tree.root_modules {
                if module.is_public {
                    content.push_str(&format!("pub use {}::*;\n", module.name));
                }
            }
        }
    } else {
        // No mod.wj - auto-generate everything
        content.push_str("// Auto-discovered modules\n");
        for module in &module_tree.root_modules {
            content.push_str(&format!("pub mod {};\n", module.name));
        }
        
        content.push_str("\n// Auto-generated re-exports\n");
        for module in &module_tree.root_modules {
            content.push_str(&format!("pub use {}::*;\n", module.name));
        }
    }
    
    Ok(content)
}

/// Generate mod.rs content for a submodule directory
pub fn generate_mod_rs_for_submodule(module: &Module) -> Result<String> {
    let mut content = String::from("// Auto-generated mod.rs by Windjammer\n\n");
    
    // Check if this module has a mod.wj
    let mod_wj_path = module.path.join("mod.wj");
    let has_mod_wj = mod_wj_path.exists();
    
    if has_mod_wj {
        // Parse mod.wj to extract declarations
        let mod_content = fs::read_to_string(&mod_wj_path)?;
        let (pub_mods, pub_uses) = parse_mod_declarations(&mod_content);
        
        // Generate pub mod declarations for submodules
        content.push_str("// Submodule declarations\n");
        for submodule in &module.submodules {
            if pub_mods.is_empty() || pub_mods.contains(&submodule.name) {
                if submodule.is_public {
                    content.push_str(&format!("pub mod {};\n", submodule.name));
                } else {
                    content.push_str(&format!("mod {};\n", submodule.name));
                }
            }
        }
        
        // Add re-exports if specified
        if !pub_uses.is_empty() {
            content.push_str("\n// Re-exports\n");
            for pub_use in pub_uses {
                content.push_str(&format!("pub use {};\n", pub_use));
            }
        }
    } else {
        // No mod.wj - auto-generate declarations
        content.push_str("// Auto-discovered submodules\n");
        for submodule in &module.submodules {
            content.push_str(&format!("pub mod {};\n", submodule.name));
        }
        
        content.push_str("\n// Auto-generated re-exports\n");
        for submodule in &module.submodules {
            content.push_str(&format!("pub use {}::*;\n", submodule.name));
        }
    }
    
    Ok(content)
}

/// Parse mod.wj to extract pub mod and pub use declarations
///
/// Returns: (pub_mod_names, pub_use_paths)
fn parse_mod_declarations(content: &str) -> (Vec<String>, Vec<String>) {
    let mut pub_mods = Vec::new();
    let mut pub_uses = Vec::new();
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        // Match: pub mod <name>
        if trimmed.starts_with("pub mod ") {
            if let Some(name) = trimmed.strip_prefix("pub mod ")
                .and_then(|s| s.split_whitespace().next()) {
                pub_mods.push(name.to_string());
            }
        }
        // Match: pub use <path>
        else if trimmed.starts_with("pub use ") {
            if let Some(path) = trimmed.strip_prefix("pub use ") {
                // Remove trailing semicolon if present
                let path = path.trim_end_matches(';').trim();
                pub_uses.push(path.to_string());
            }
        }
    }
    
    (pub_mods, pub_uses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_dir(files: &[(&str, &str)]) -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        
        for (path, content) in files {
            let full_path = temp_dir.path().join(path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&full_path, content).unwrap();
        }
        
        temp_dir
    }

    #[test]
    fn test_parse_mod_declarations() {
        let content = r#"
pub mod math
pub mod rendering
pub use math::Vec2
pub use math::Vec3
pub use rendering::Color
"#;
        
        let (pub_mods, pub_uses) = parse_mod_declarations(content);
        
        assert_eq!(pub_mods, vec!["math", "rendering"]);
        assert_eq!(pub_uses, vec!["math::Vec2", "math::Vec3", "rendering::Color"]);
    }

    #[test]
    fn test_discover_flat_modules() {
        let temp_dir = create_test_dir(&[
            ("vec2.wj", "pub struct Vec2 { pub x: f64, pub y: f64 }"),
            ("vec3.wj", "pub struct Vec3 { pub x: f64, pub y: f64, pub z: f64 }"),
        ]);

        let tree = discover_nested_modules(temp_dir.path()).unwrap();
        
        assert_eq!(tree.root_modules.len(), 2);
        assert!(tree.has_module(&["vec2"]));
        assert!(tree.has_module(&["vec3"]));
    }

    #[test]
    fn test_discover_nested_modules() {
        let temp_dir = create_test_dir(&[
            ("mod.wj", "pub mod math\npub mod rendering"),
            ("math/mod.wj", "pub mod vec2\npub mod vec3"),
            ("math/vec2.wj", "pub struct Vec2 { pub x: f64, pub y: f64 }"),
            ("math/vec3.wj", "pub struct Vec3 { pub x: f64, pub y: f64, pub z: f64 }"),
            ("rendering/color.wj", "pub struct Color { pub r: u8, pub g: u8, pub b: u8 }"),
        ]);

        let tree = discover_nested_modules(temp_dir.path()).unwrap();
        
        assert_eq!(tree.root_modules.len(), 2);
        assert!(tree.has_module(&["math"]));
        assert!(tree.has_module(&["math", "vec2"]));
        assert!(tree.has_module(&["math", "vec3"]));
        assert!(tree.has_module(&["rendering"]));
        assert!(tree.has_module(&["rendering", "color"]));
    }

    #[test]
    fn test_generate_lib_rs_with_explicit_declarations() {
        let temp_dir = create_test_dir(&[
            ("mod.wj", "pub mod math\npub use math::Vec2\npub use math::Vec3"),
            ("math/vec2.wj", "pub struct Vec2 { pub x: f64, pub y: f64 }"),
            ("math/vec3.wj", "pub struct Vec3 { pub x: f64, pub y: f64, pub z: f64 }"),
        ]);

        let tree = discover_nested_modules(temp_dir.path()).unwrap();
        let lib_rs = generate_lib_rs(&tree).unwrap();
        
        assert!(lib_rs.contains("pub mod math;"));
        assert!(lib_rs.contains("pub use math::Vec2;"));
        assert!(lib_rs.contains("pub use math::Vec3;"));
        assert!(!lib_rs.contains("pub use math::*;"), "Should use explicit re-exports, not wildcard");
    }
}

