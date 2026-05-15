//! Resolving paths to hand-written Rust modules (FFI, sibling `mod.rs`, etc.).

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::Result;

use super::module_graph::{Module, ModuleTree};

fn collect_all_module_names(modules: &[Module], names: &mut HashSet<String>) {
    for m in modules {
        names.insert(m.name.clone());
        if !m.submodules.is_empty() {
            collect_all_module_names(&m.submodules, names);
        }
    }
}

/// Returns only modules that are NOT in the generated module tree.
///
/// Discover hand-written Rust modules in the project root
///
/// THE WINDJAMMER WAY: Allow seamless FFI/interop!
/// Users can provide hand-written Rust code (like ffi.rs or ffi/mod.rs)
/// in the project root alongside src/, and it will be automatically
/// integrated with generated code.
///
/// This enables:
/// - FFI bindings to C/C++
/// - Direct Rust interop
/// - Performance-critical code in pure Rust
/// - Gradual migration from Rust to Windjammer
pub(crate) fn discover_hand_written_modules(
    project_root: &Path,
    module_tree: &ModuleTree,
    output_dir: &Path,
) -> Result<Vec<String>> {
    let mut modules = Vec::new();

    if !project_root.exists() {
        return Ok(modules);
    }

    // Build set of ALL generated module names (including nested submodules) for quick lookup.
    // This prevents stale copies of nested modules (e.g., sundering/player/) in src/ from
    // being picked up as "hand-written" top-level modules.
    let mut generated_names = HashSet::new();
    collect_all_module_names(&module_tree.root_modules, &mut generated_names);

    // THE WINDJAMMER WAY: Rust convention is to put library code in src/
    // So we need to check both project_root/ and project_root/src/ for hand-written modules
    let mut search_dirs = vec![project_root.to_path_buf()];

    // Only add src/ if it's different from project_root
    let src_dir = project_root.join("src");
    if src_dir.exists() && src_dir != project_root {
        search_dirs.push(src_dir);
    }

    // BUG #12 FIX: Also search the parent directory of output_dir for sibling modules
    // Example: If output is src/components/generated/, search src/components/ for siblings
    // like platform.rs
    if let Some(output_parent) = output_dir.parent() {
        if output_parent != project_root && output_parent != project_root.join("src") {
            search_dirs.push(output_parent.to_path_buf());
        }
    }

    for search_dir in &search_dirs {
        if !search_dir.exists() {
            continue;
        }

        // When search_dir IS the output_dir, flat .rs files are either generated
        // (already in module tree) or stale (from previous builds). Only discover
        // directory modules (like ffi/) in this case.
        let search_is_output = {
            let a = search_dir
                .canonicalize()
                .unwrap_or_else(|_| search_dir.clone());
            let b = output_dir
                .canonicalize()
                .unwrap_or_else(|_| output_dir.to_path_buf());
            a == b
        };

        let is_output_parent = output_dir.parent() == Some(search_dir.as_path());
        let needs_scope_filter = if is_output_parent {
            // Searching immediate parent of output - include siblings, no filtering
            false
        } else if output_dir.strip_prefix(search_dir).is_ok() {
            // Output is within search_dir, but not immediate child
            // Apply filtering to avoid including unrelated files
            if let Ok(rel) = output_dir.strip_prefix(search_dir) {
                // If rel has 2+ components, we're searching an ancestor directory
                rel.components().count() >= 2
            } else {
                false
            }
        } else {
            false
        };

        for entry in fs::read_dir(search_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                // When search_dir IS the output directory, skip ALL flat .rs files.
                // They are either generated (in the module tree) or stale artifacts.
                // Only directory modules (like ffi/) are legitimate hand-written modules.
                if search_is_output {
                    continue;
                }

                if let Some(name) = path.file_stem() {
                    let name_str = name.to_string_lossy().to_string();

                    // Skip .rs files that have corresponding subdirectories
                    // Example: Skip events.rs if events/ directory exists
                    // (these are module parent files that won't work in generated context)
                    let corresponding_dir = search_dir.join(&name_str);
                    if corresponding_dir.exists() && corresponding_dir.is_dir() {
                        continue; // Skip this file - it's a module parent
                    }

                    // BUG #12 FIX: Apply scope filtering
                    if needs_scope_filter {
                        // Output is within src/ (e.g., src/components/generated/)
                        // Only include files that are within the same subdirectory tree
                        if let Ok(rel_output) = output_dir.strip_prefix(search_dir) {
                            if let Ok(rel_path) = path.strip_prefix(search_dir) {
                                // Check if the file has a parent directory within search_dir
                                // e.g., src/components/platform.rs -> parent is "components"
                                // e.g., src/app.rs -> no parent (directly in search_dir)
                                if let Some(file_parent) = rel_path.parent() {
                                    if file_parent == Path::new("") {
                                        // File is directly in search_dir (e.g., src/app.rs)
                                        // But output is in a subdirectory (e.g., src/components/generated/)
                                        // Skip these top-level files
                                        continue;
                                    }

                                    // Get the first component of the output and file paths
                                    // e.g., for src/components/generated/ -> "components"
                                    // e.g., for src/components/platform.rs -> "components"
                                    let output_first_component = rel_output.components().next();
                                    let path_first_component = file_parent.components().next();

                                    // Only include if they share the same first component
                                    if let (Some(output_comp), Some(path_comp)) =
                                        (output_first_component, path_first_component)
                                    {
                                        if output_comp != path_comp {
                                            continue; // Skip - different subdirectory tree
                                        }
                                    } else {
                                        continue; // Skip - incompatible paths
                                    }
                                } else {
                                    // No parent -> file is at root (shouldn't happen for files in search_dir)
                                    continue;
                                }
                            }
                        }
                    } else if let Some(output_parent) = output_dir.parent() {
                        // When searching output_dir.parent(), only include files
                        // that are SIBLINGS of the output directory
                        if *search_dir == output_parent {
                            // We're searching the immediate parent of output_dir
                            // Make sure the file is NOT from a parent of output_parent
                            if let Some(file_parent) = path.parent() {
                                if file_parent != output_parent {
                                    continue; // Skip - file is not a direct sibling
                                }
                            }
                        }
                    }

                    // Skip lib.rs, mod.rs, main.rs, and generated modules
                    if name_str != "lib"
                        && name_str != "mod"
                        && name_str != "main"
                        && !generated_names.contains(&name_str)
                        && path.extension().and_then(|s| s.to_str()) == Some("rs")
                        && !modules.contains(&name_str)
                    {
                        // Skip files that look auto-generated by the Windjammer compiler
                        // (stale flat .rs files from previous builds)
                        let is_auto_generated = if let Ok(content) = fs::read_to_string(&path) {
                            content.starts_with("#[allow(unused_imports)]")
                                || content.starts_with("// Auto-generated")
                        } else {
                            false
                        };
                        if !is_auto_generated {
                            modules.push(name_str);
                        }
                    }
                }
            } else if path.is_dir() {
                // CRITICAL FIX: Don't declare directories that are ancestors of output_dir
                // Example: Don't declare "pub mod components;" when output is src/components/generated/
                if let (Ok(canonical_dir), Ok(canonical_output)) =
                    (path.canonicalize(), output_dir.canonicalize())
                {
                    if canonical_output.starts_with(&canonical_dir) {
                        continue; // Skip - this directory is an ancestor of output
                    }
                }

                // BUG #12 FIX: Apply same scope filtering to directories as we do for files
                if needs_scope_filter {
                    // Output is within search_dir (e.g., src/components/generated/)
                    // Only include directories that are within the same subdirectory tree
                    if let Ok(rel_output) = output_dir.strip_prefix(search_dir) {
                        if let Ok(rel_path) = path.strip_prefix(search_dir) {
                            // Check if the directory has a parent within search_dir
                            // e.g., src/components/platform/ -> parent is "components"
                            // e.g., src/platform/ -> no parent (directly in search_dir)
                            if let Some(dir_parent) = rel_path.parent() {
                                if dir_parent == Path::new("") {
                                    // Directory is directly in search_dir (e.g., src/platform/)
                                    // But output is in a subdirectory (e.g., src/components/generated/)
                                    // Skip these top-level directories
                                    continue;
                                }

                                // Get the first component of the output and directory paths
                                // e.g., for src/components/generated/ -> "components"
                                // e.g., for src/components/platform/ -> "components"
                                let output_first_component = rel_output.components().next();
                                let path_first_component = dir_parent.components().next();

                                // Only include if they share the same first component
                                if let (Some(output_comp), Some(path_comp)) =
                                    (output_first_component, path_first_component)
                                {
                                    if output_comp != path_comp {
                                        continue; // Skip - different subdirectory tree
                                    }
                                } else {
                                    continue; // Skip - incompatible paths
                                }
                            } else {
                                // No parent -> directory is at root (shouldn't happen)
                                continue;
                            }
                        }
                    }
                } else if let Some(output_parent) = output_dir.parent() {
                    // When searching output_dir.parent(), only include directories
                    // that are SIBLINGS of the output directory
                    if *search_dir == output_parent {
                        // We're searching the immediate parent of output_dir
                        // Make sure the directory is NOT from a parent of output_parent
                        if let Some(dir_parent) = path.parent() {
                            if dir_parent != output_parent {
                                continue; // Skip - directory is not a direct sibling
                            }
                        }
                    }
                }

                // Check if directory has a mod.rs (but skip common non-FFI directories)
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let skip_dirs = [
                    "target",
                    "build",
                    "generated",
                    "dist",
                    "node_modules",
                    ".git",
                    "lib",
                    "tests_build",
                    "test_output",
                    "test_scenarios",
                    "examples",
                    "benches",
                    "wj-plugins",
                ];
                if !skip_dirs.contains(&dir_name) {
                    let mod_rs = path.join("mod.rs");
                    if mod_rs.exists() {
                        if let Some(name) = path.file_name() {
                            let name_str = name.to_string_lossy().to_string();
                            // Only include if not a generated module
                            if !generated_names.contains(&name_str) && !modules.contains(&name_str)
                            {
                                modules.push(name_str);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(modules)
}
