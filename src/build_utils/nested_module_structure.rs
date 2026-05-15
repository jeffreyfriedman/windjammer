//! Discover nested Windjammer modules and emit `lib.rs` / `mod.rs` plus submodule trees.

use anyhow::Result;
use std::path::Path;

pub fn generate_nested_module_structure(source_dir: &Path, output_dir: &Path) -> Result<()> {
    use crate::module_system::{
        discover_nested_modules, generate_lib_rs, generate_mod_rs_for_submodule,
    };
    use anyhow::Context;
    use colored::*;

    // Discover all modules in the source directory
    let module_tree =
        discover_nested_modules(source_dir).context("Failed to discover module structure")?;

    // Build set of ALL generated module names (including nested submodules)
    // Used to prevent stale copies in src/ from being treated as hand-written modules
    fn collect_all_names(
        modules: &[crate::module_system::Module],
        names: &mut std::collections::HashSet<String>,
    ) {
        for m in modules {
            names.insert(m.name.clone());
            if !m.submodules.is_empty() {
                collect_all_names(&m.submodules, names);
            }
        }
    }
    let mut all_generated_names = std::collections::HashSet::new();
    collect_all_names(&module_tree.root_modules, &mut all_generated_names);

    // Generate lib.rs (for crate root) or mod.rs (for subdirectory)
    // THE WINDJAMMER WAY: Auto-discover hand-written Rust modules (FFI/interop)
    // Look for hand-written .rs files in the project root (parent of src)
    let project_root = if let Some(parent) = source_dir.parent() {
        if parent.as_os_str().is_empty() {
            std::path::Path::new(".")
        } else {
            parent
        }
    } else {
        source_dir
    };

    // Determine if we should generate lib.rs or mod.rs
    // Generate mod.rs when output is a subdirectory (e.g., src/components/generated)
    // Generate lib.rs when output is a crate root (e.g., out/, target/generated)
    //
    // Detection heuristic:
    // - If path contains ".../src/..." with more components after src, generate mod.rs
    // - Otherwise, generate lib.rs
    let components: Vec<_> = output_dir.components().collect();
    let mut is_subdirectory = false;
    for (i, component) in components.iter().enumerate() {
        if let std::path::Component::Normal(name) = component {
            if name.to_string_lossy() == "src" && i + 1 < components.len() {
                // Path contains ".../src/..." with more components after src
                is_subdirectory = true;
                break;
            }
        }
    }

    let (module_file_name, module_file_path) = if is_subdirectory {
        ("mod.rs", output_dir.join("mod.rs"))
    } else {
        ("lib.rs", output_dir.join("lib.rs"))
    };

    let module_content = generate_lib_rs(&module_tree, project_root, output_dir)?;
    eprintln!(
        "DEBUG generate_nested: writing {} at {:?}",
        module_file_name, module_file_path
    );
    eprintln!(
        "DEBUG generate_nested: project_root={:?}, output_dir={:?}, source_dir={:?}",
        project_root, output_dir, source_dir
    );
    std::fs::write(&module_file_path, module_content)?;

    // Copy hand-written modules to output directory
    // THE WINDJAMMER WAY: Seamless FFI integration!
    // BUT: Don't overwrite generated .rs files from .wj sources!
    // Check both project_root/ and project_root/src/ (Rust convention)
    let copy_dirs = vec![project_root.to_path_buf(), project_root.join("src")];

    for copy_dir in &copy_dirs {
        if !copy_dir.exists() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(copy_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy();
                        // Copy .rs files that aren't lib.rs, mod.rs, or build.rs
                        // build.rs is a Cargo build script and should not be copied to output
                        if name_str.ends_with(".rs")
                            && name_str != "lib.rs"
                            && name_str != "mod.rs"
                            && name_str != "build.rs"
                        {
                            let stem = path.file_stem().unwrap().to_string_lossy();

                            // Don't copy main.rs (not a module)
                            if stem == "main" {
                                continue;
                            }

                            // Don't copy .rs files that match a generated module name
                            if all_generated_names.contains(stem.as_ref()) {
                                continue;
                            }

                            // Don't copy auto-generated files (stale from previous builds)
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if content.starts_with("#[allow(unused_imports)]")
                                    || content.starts_with("// Auto-generated")
                                {
                                    continue;
                                }
                            }

                            // Don't copy .rs files with a corresponding .wj source
                            let corresponding_wj = source_dir.join(format!("{}.wj", stem));
                            if corresponding_wj.exists() {
                                continue;
                            }

                            // CRITICAL FIX: Don't copy .rs files that have corresponding subdirectories
                            // Example: Don't copy events.rs if events/ directory exists
                            // (events.rs declares submodules which won't work in generated context)
                            let corresponding_dir = path.parent().unwrap().join(stem.as_ref());
                            if corresponding_dir.exists() && corresponding_dir.is_dir() {
                                continue; // Skip copying - this is a module parent file
                            }

                            // BUG #12 FIX: Don't copy out-of-scope .rs files
                            // -----------------------------------------------
                            // When output is a subdirectory like src/components/generated/,
                            // we should NOT copy .rs files from the parent src/ directory
                            // (like src/app.rs, src/examples_wasm.rs).
                            //
                            // These are top-level modules declared in src/lib.rs, not part
                            // of the generated components.
                            //
                            // Only copy .rs files that are within the same subdirectory tree
                            // as the output directory.
                            let should_copy = if copy_dir.ends_with("src") {
                                // When scanning src/, check if output is also in src/
                                if let Ok(rel_output) = output_dir.strip_prefix(copy_dir) {
                                    // Output is within src/ (e.g., src/components/generated/)
                                    // Check if this .rs file is within the output tree
                                    if let Ok(rel_path) = path.strip_prefix(copy_dir) {
                                        // Get the first component of the output relative path
                                        // e.g., for src/components/generated/ -> "components"
                                        let output_first_component = rel_output.components().next();
                                        let path_first_component = rel_path.components().next();

                                        // Only copy if they share the same first component
                                        // (i.e., they're in the same subdirectory tree)
                                        // OR if the file is in the immediate directory (no subdirectory)
                                        if let (Some(output_comp), Some(path_comp)) =
                                            (output_first_component, path_first_component)
                                        {
                                            output_comp == path_comp
                                        } else {
                                            // File is directly in src/ (e.g., src/app.rs)
                                            // Don't copy - these are top-level modules
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                } else {
                                    // Output is NOT within src/, so we're at project root scope
                                    // Copy all .rs files from src/ (they're in scope)
                                    true
                                }
                            } else {
                                // Not scanning src/, so copy (project root FFI files)
                                true
                            };

                            if should_copy {
                                // Only copy hand-written .rs files (like ffi.rs)
                                let dest = output_dir.join(name);
                                if let Err(e) = std::fs::copy(&path, &dest) {
                                    eprintln!("Warning: Failed to copy {}: {}", name_str, e);
                                }
                            } else {
                                eprintln!(
                                    "⏭️  Skipping out-of-scope file: {} (not within output tree)",
                                    path.display()
                                );
                            }
                        }
                    }
                } else if path.is_dir() {
                    // THE WINDJAMMER WAY: Copy directories with hand-written Rust (like ffi/)
                    if let Some(dir_name) = path.file_name() {
                        let dir_name_str = dir_name.to_string_lossy();
                        let skip_dirs = [
                            "target",
                            "build",
                            "generated",
                            "dist",
                            "node_modules",
                            ".git",
                            "tests_build",
                            "test_output",
                            "test_scenarios",
                            "examples",
                            "benches",
                            "lib",
                        ];

                        if !skip_dirs.contains(&dir_name_str.as_ref()) {
                            // Don't copy directories that correspond to ANY generated module
                            // (including nested submodules like sundering/player → player)
                            if all_generated_names.contains(dir_name_str.as_ref()) {
                                continue;
                            }

                            // Also check for a corresponding .wj directory in src
                            let corresponding_wj_dir = source_dir.join(dir_name_str.as_ref());
                            if corresponding_wj_dir.exists() && corresponding_wj_dir.is_dir() {
                                continue;
                            }

                            // CRITICAL FIX: Don't copy directories that contain the output directory!
                            // This prevents infinite recursion when output is inside the source tree
                            // Example: Don't copy src/components/ when output is src/components/generated/
                            if let Ok(canonical_dir) = path.canonicalize() {
                                if let Ok(canonical_output) = output_dir.canonicalize() {
                                    if canonical_output.starts_with(&canonical_dir) {
                                        continue; // Skip copying - this dir contains the output directory
                                    }
                                }
                            }

                            // Check if this directory has a mod.rs (it's a Rust module)
                            let mod_rs = path.join("mod.rs");
                            if mod_rs.exists() {
                                // Bug #9B FIX: Don't copy out-of-scope hand-written modules
                                // --------------------------------------------------------
                                // When output is a subdirectory like src/components/generated/,
                                // we should NOT copy modules from src/ (like src/events/).
                                //
                                // Only copy modules that are:
                                // 1. In the project root (for FFI interop), OR
                                // 2. Would be copied by the module system itself
                                //
                                // Heuristic: If the module's parent directory is NOT the same as
                                // copy_dir (the directory we're scanning), it's out of scope.
                                //
                                // Example:
                                // - copy_dir: project_root/src/
                                // - path: project_root/src/events/
                                // - output_dir: project_root/src/components/generated/
                                // - Skip because events/ is NOT within components/generated/ tree

                                let should_copy = if copy_dir.ends_with("src") {
                                    // When scanning src/, check if output is also in src/
                                    // If output is src/components/generated/, only copy modules
                                    // that are within the components/ tree
                                    if let Ok(rel_output) = output_dir.strip_prefix(copy_dir) {
                                        // Output is within src/ (e.g., src/components/generated/)
                                        // Check if this module is within the output tree
                                        if let Ok(rel_path) = path.strip_prefix(copy_dir) {
                                            // Get the first component of the output relative path
                                            // e.g., for src/components/generated/ -> "components"
                                            let output_first_component =
                                                rel_output.components().next();
                                            let path_first_component = rel_path.components().next();

                                            // Only copy if they share the same first component
                                            // (i.e., they're in the same subdirectory tree)
                                            output_first_component == path_first_component
                                        } else {
                                            false
                                        }
                                    } else {
                                        // Output is NOT within src/, so we're at project root scope
                                        // Copy all modules from src/ (they're in scope)
                                        true
                                    }
                                } else {
                                    // Not scanning src/, so copy (project root FFI modules)
                                    true
                                };

                                if should_copy {
                                    let dest_dir = output_dir.join(dir_name);
                                    if let Err(e) =
                                        crate::test_runner::copy_dir_recursive(&path, &dest_dir)
                                    {
                                        eprintln!(
                                            "Warning: Failed to copy directory {}: {}",
                                            dir_name_str, e
                                        );
                                    }
                                } else {
                                    eprintln!(
                                        "⏭️  Skipping out-of-scope module: {} (not within output tree)",
                                        path.display()
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!(
        "{} Generated {} with {} top-level modules",
        "✓".green(),
        module_file_name,
        module_tree.root_modules.len()
    );

    // Recursively generate mod.rs for each directory module
    fn generate_mod_rs_recursive(
        module: &crate::module_system::Module,
        output_dir: &Path,
    ) -> Result<()> {
        if module.is_directory && !module.submodules.is_empty() {
            let module_output_dir = output_dir.join(&module.name);
            std::fs::create_dir_all(&module_output_dir)?;
            let mod_rs_content = generate_mod_rs_for_submodule(module, &module_output_dir)?;
            let mod_rs_path = module_output_dir.join("mod.rs");
            std::fs::write(&mod_rs_path, mod_rs_content)?;

            // Recursively generate for submodules
            for submodule in &module.submodules {
                generate_mod_rs_recursive(submodule, &module_output_dir)?;
            }
        }
        Ok(())
    }

    // Generate mod.rs for all directory modules
    for module in &module_tree.root_modules {
        generate_mod_rs_recursive(module, output_dir)?;
    }

    Ok(())
}
