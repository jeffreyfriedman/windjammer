//! Module tree discovery — directory layout and nested module structure.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

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
    for entry in fs::read_dir(dir_path)
        .with_context(|| format!("Failed to read directory: {:?}", dir_path))?
    {
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

        // THE WINDJAMMER WAY: Only include directories that have content
        // Skip empty directories to avoid "file not found for module" errors
        let has_content = has_mod_wj || !submodules.is_empty();

        if has_content {
            modules.push(Module {
                name: dir_name.clone(),
                path: dir_path,
                is_directory: true,
                submodules,
                has_mod_wj,
                is_public: true, // TODO: Parse mod.wj to determine visibility
            });
        }
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
