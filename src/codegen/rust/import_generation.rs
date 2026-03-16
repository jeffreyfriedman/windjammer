//! Import/use statement generation for Rust code.
//!
//! This module handles the conversion of Windjammer import paths to Rust `use` statements.
//! It resolves crate::, std::, relative (./, ../), and sibling module imports, including
//! support for nested output directory structures (e.g., src/generated/core/commands/).

use super::CodeGenerator;

impl CodeGenerator<'_> {
    /// Calculate the import prefix for cross-module imports based on output file nesting
    /// Returns the number of directory levels to go up (for super:: prefixes)
    fn get_import_prefix_for_nested_output(&self) -> Option<usize> {
        if self.current_output_file.as_os_str().is_empty() {
            return None;
        }

        // Count directory levels by checking parent directories
        // For src/generated/core/commands/command.rs:
        // - command.rs (file)
        // - commands/ (parent 1)
        // - core/ (parent 2)
        // - generated/ (parent 3 - this is our module root)
        // - src/ (parent 4)
        // So from core/commands/ we need to go up 2 levels to get to generated/

        // Get the path and count parent directories excluding the filename
        let mut parent = self.current_output_file.parent();
        let mut depth = 0;

        while let Some(p) = parent {
            let dir_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Stop when we hit a known module root directory
            // These are directories that typically contain the generated modules
            if dir_name == "generated"
                || dir_name == "build"
                || dir_name == "out"
                || dir_name == "src"
            {
                // Found module root - return current depth
                if depth > 0 {
                    return Some(depth);
                }
                break;
            }

            depth += 1;
            parent = p.parent();
        }

        None
    }

    fn get_module_root_name(&self) -> Option<String> {
        // Walk up the directory tree to find the module root name
        // KEY DISTINCTION:
        // - If directory has lib.rs: it's the crate root -> return None
        // - If directory is inside src/ of another crate: it's a submodule -> return dir name
        // - Known output dirs (build, generated, out) are always the crate root
        //   even if lib.rs hasn't been generated yet (chicken-and-egg with CLI)

        let mut parent = self.current_output_file.parent();

        while let Some(p) = parent {
            let dir_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if dir_name == "generated" || dir_name == "build" || dir_name == "out" {
                // If this directory contains lib.rs, it IS the crate root
                if p.join("lib.rs").exists() {
                    return None;
                }

                // TDD FIX: Only treat as a submodule if this output directory is
                // INSIDE another crate's src/ directory. A parent lib.rs that's NOT
                // in a src/ directory belongs to a DIFFERENT crate (sibling project),
                // not the one we're generating into.
                if let Some(parent_of_p) = p.parent() {
                    let parent_name = parent_of_p
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    if parent_name == "src" && parent_of_p.join("lib.rs").exists() {
                        return Some(dir_name.to_string());
                    }
                }

                // Known output directory without lib.rs yet - it IS the crate root.
                // lib.rs will be generated later by the CLI.
                return None;
            }

            if dir_name == "src" {
                break;
            }

            parent = p.parent();
        }

        None
    }

    pub(super) fn generate_use(&self, path: &[String], alias: Option<&str>) -> String {
        if path.is_empty() {
            return String::new();
        }

        let full_path = path.join(".");

        // SPECIAL CASE: Handle crate:: imports when in nested module output
        // Examples:
        // - use crate::scene::{A, B} -> use crate::generated::scene::{A, B}
        // - use crate::scene::Scene -> use crate::generated::scene::Scene
        // This applies to both braced and non-braced imports
        if full_path.starts_with("crate::") || full_path.starts_with("crate.") {
            // Find the module root (e.g., "generated", "build", "out")
            let module_root = if self.is_module {
                self.get_module_root_name()
            } else {
                None
            };

            let rewritten = if let Some(root_name) = module_root {
                // Normalize to use :: separator
                let normalized = full_path.replace('.', "::");
                // Rewrite: crate::scene::X -> crate::generated::scene::X
                let path_without_crate = normalized.strip_prefix("crate::").unwrap();
                format!("crate::{}::{}", root_name, path_without_crate)
            } else {
                // No module root detected, keep as-is
                full_path.replace('.', "::")
            };

            // TDD FIX: Preserve alias for crate:: imports
            if let Some(alias_name) = alias {
                return format!("use {} as {};\n", rewritten, alias_name);
            } else {
                return format!("use {};\n", rewritten);
            }
        }

        // Handle stdlib imports FIRST (before glob handling)
        // This ensures std::ui::*, std::fs::*, etc. are properly skipped
        if full_path.starts_with("std::") || full_path.starts_with("std.") {
            // Normalize to use :: separator
            let normalized = full_path.replace('.', "::");
            let module_name = normalized.strip_prefix("std::").unwrap();

            // Strip glob suffix if present for checking
            let module_base = module_name.strip_suffix("::*").unwrap_or(module_name);

            // Handle Rust stdlib modules that should NOT be mapped to windjammer_runtime
            // These are native Rust modules that should be used directly
            if module_base.starts_with("collections")
                || module_base.starts_with("cmp")
                || module_base.starts_with("ops")
                || module_base == "ops"
            {
                // TDD FIX: Pass through to Rust's std library with alias support
                if let Some(alias_name) = alias {
                    return format!("use std::{} as {};\n", module_name, alias_name);
                } else {
                    return format!("use std::{};\n", module_name);
                }
            }

            // Handle UI framework - skip explicit import (handled by implicit imports)
            if module_base == "ui" || module_base.starts_with("ui::") {
                // UI framework is handled by implicit imports from windjammer-ui crate
                // Don't generate an explicit import here
                return String::new();
            }

            // Handle Game framework - skip explicit import (handled by implicit imports)
            if module_base == "game" || module_base.starts_with("game::") {
                // Game framework is handled by implicit imports from windjammer-game-framework crate
                // Don't generate an explicit import here
                return String::new();
            }

            // Handle Tauri framework - skip explicit import (functions are generated inline)
            if module_base == "tauri" || module_base.starts_with("tauri::") {
                // Tauri functions are handled by compiler codegen (generate_tauri_invoke)
                // Don't generate an explicit import here
                return String::new();
            }

            // Handle platform APIs - skip explicit import (handled by implicit imports)
            if module_base == "fs"
                || module_base.starts_with("fs::")
                || module_base == "process"
                || module_base.starts_with("process::")
                || module_base == "dialog"
                || module_base.starts_with("dialog::")
                || module_base == "env"
                || module_base.starts_with("env::")
                || module_base == "encoding"
                || module_base.starts_with("encoding::")
                || module_base == "compute"
                || module_base.starts_with("compute::")
                || module_base == "net"
                || module_base.starts_with("net::")
                || module_base == "http"
                || module_base.starts_with("http::")
                || module_base == "storage"
                || module_base.starts_with("storage::")
            {
                // Platform APIs are handled by implicit imports (platform-specific)
                // Don't generate an explicit import here
                return String::new();
            }

            // Map to windjammer_runtime (all stdlib modules are now implemented!)
            let rust_import = match module_name {
                // Core modules
                "http" => "windjammer_runtime::http",
                "mime" => "windjammer_runtime::mime",
                "json" => "windjammer_runtime::json",

                // Additional modules
                "async" => "windjammer_runtime::async_runtime",
                "cli" => "windjammer_runtime::cli",
                "crypto" => "windjammer_runtime::crypto",
                "csv" => "windjammer_runtime::csv_mod",
                "db" => "windjammer_runtime::db",
                "log" => "windjammer_runtime::log_mod",
                "math" => "windjammer_runtime::math",
                "random" => "windjammer_runtime::random",
                "regex" => "windjammer_runtime::regex_mod",
                "strings" => "windjammer_runtime::strings",
                "testing" => "windjammer_runtime::testing",
                "time" => "windjammer_runtime::time",
                // "ui" is handled by implicit imports (windjammer-ui crate), not runtime
                "game" => "windjammer_runtime::game",

                _ => {
                    // Unknown module - try windjammer_runtime
                    return format!("use windjammer_runtime::{};\n", module_name);
                }
            };

            if let Some(alias_name) = alias {
                return format!("use {} as {};\n", rust_import, alias_name);
            } else {
                // For _mod suffixed modules (log_mod, regex_mod), alias back to the original name
                // AND import any public types they export
                if rust_import.ends_with("_mod") {
                    let original_name = rust_import
                        .strip_suffix("_mod")
                        .and_then(|s| s.split("::").last())
                        .unwrap_or(rust_import);

                    let mut result = format!("use {} as {};\n", rust_import, original_name);

                    // Import types for modules that export them
                    match original_name {
                        "regex" => {
                            result.push_str(&format!("use {}::Regex;\n", rust_import));
                        }
                        "time" => {
                            result.push_str(&format!(
                                "use {}::{{Duration, Instant}};\n",
                                rust_import
                            ));
                        }
                        _ => {}
                    }

                    return result;
                }
                // Import the module itself (not glob) to keep module-qualified paths
                // For types like Duration, we'll need explicit imports or full paths
                return format!("use {};\n", rust_import);
            }
        }

        // Skip bare "std" imports
        if full_path == "std" {
            return String::new();
        }

        // Handle glob imports for non-stdlib modules: module.submodule.* -> use module::submodule::*;
        if full_path.ends_with(".*") {
            let path_without_glob = full_path.strip_suffix(".*").unwrap();
            // Replace dots with :: but remove any trailing ::
            let rust_path = path_without_glob
                .replace('.', "::")
                .trim_end_matches("::")
                .to_string();
            return format!("use {}::*;\n", rust_path);
        }

        // Handle braced imports: module::{A, B, C} or module.{A, B, C}
        if (full_path.contains("::{") || full_path.contains(".{")) && full_path.contains('}') {
            // Try :: separator first, then . separator
            if let Some((base, items)) = full_path.split_once("::{") {
                return format!("use {}::{{{};\n", base, items);
            } else if let Some((base, items)) = full_path.split_once(".{") {
                let rust_base = base.replace('.', "::");
                return format!("use {}::{{{};\n", rust_base, items);
            }
        }

        // Handle relative imports: ./utils or ../utils or ./config::Config
        if full_path.starts_with("./") || full_path.starts_with("../") {
            // Strip the leading ./ or ../
            let stripped = full_path
                .strip_prefix("./")
                .or_else(|| full_path.strip_prefix("../"))
                .unwrap_or(&full_path);

            // Check if this is importing a specific item (e.g., ./config::Config)
            if stripped.contains("::") {
                // Split into module path and item
                let rust_path = stripped.replace('/', "::");
                // Check if the last segment looks like a type (uppercase)
                let segments: Vec<&str> = rust_path.split("::").collect();
                if let Some(last) = segments.last() {
                    if last.chars().next().is_some_and(|c| c.is_uppercase()) {
                        // Importing a specific type: ./config::Config -> use crate::config::Config;
                        return format!("use crate::{};\n", rust_path);
                    }
                }
                // For crate::module imports, just import the module (not ::*)
                // This allows qualified usage like module::func() in the code
                return format!("use crate::{};\n", rust_path);
            } else {
                // Module import: ./config
                // In the main entry point (is_module=false), modules are already in scope via pub mod declarations
                // In submodules (is_module=true), we need to explicitly use sibling modules
                let module_name = stripped.split('/').next_back().unwrap_or(stripped);
                if let Some(alias_name) = alias {
                    return format!("use crate::{} as {};\n", module_name, alias_name);
                } else if self.is_module {
                    // In a module, we need to explicitly use sibling modules
                    return format!("use crate::{};\n", module_name);
                } else {
                    // In main entry point, modules are already in scope
                    return String::new();
                }
            }
        }

        // Convert Windjammer's Go-style imports to Rust imports
        // Heuristic: If the last segment starts with an uppercase letter, it's likely a type/struct
        // Otherwise, it's a module and we should add ::*
        let rust_path = full_path.replace('.', "::");

        // TDD FIX: Handle imports from sibling modules (Part 2 - Nested Import Bug)
        // When in a subdirectory (e.g., rendering/sprite.wj) and importing a sibling (texture::Texture),
        // we need to detect this and rewrite to super::texture::Texture
        //
        // Detection strategy:
        // 1. Check if we're in a subdirectory (output_file contains a directory separator)
        // 2. Check if the import is bare (no std::, crate::, super:: prefix)
        // 3. Assume it's a sibling module and use super:: prefix
        //
        // THE WINDJAMMER WAY: Smart defaults that work 99% of the time
        // TDD FIX: Check for both Unix (/) and Windows (\) path separators
        let is_in_subdirectory = self
            .current_output_file
            .to_str()
            .map(|s| s.contains('/') || s.contains('\\'))
            .unwrap_or(false);

        // TDD FIX: Detect imports from parent module's re-exports
        // When in rendering/sprite.wj and seeing "use rendering::Texture",
        // this means the parent module's re-export, so convert to "use super::Texture"
        if is_in_subdirectory {
            if let Some(parent_dir) = self
                .current_output_file
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
            {
                // Check if the import starts with our parent directory name
                if rust_path.starts_with(&format!("{}::", parent_dir)) {
                    // Strip the parent directory name and use super:: instead
                    let path_without_parent = rust_path
                        .strip_prefix(&format!("{}::", parent_dir))
                        .unwrap();
                    return format!("use super::{};\n", path_without_parent);
                }
            }
        }

        // BUGFIX: Handle imports from sibling modules (flat directory structure)
        // When importing from common module names like math, rendering, collision2d, etc.,
        // these are sibling files in src/generated/, so use super:: instead of absolute paths
        //
        // IMPORTANT: Distinguish between:
        // 1. Directory prefixes (math, rendering, physics) - should be handled with crate::
        // 2. Actual module files (texture_atlas, sprite_region) - should be handled with super::
        // THE WINDJAMMER WAY: With nested module system (lib.rs), use crate:: for cross-directory imports
        // Only use super:: for same-directory imports
        let common_sibling_modules = ["vec2", "vec3", "vec4", "mat4", "quat", "color"];

        // Extract first segment early so we can use it in multiple places
        let first_segment = rust_path.split("::").next().unwrap_or("");

        // TDD FIX: Dynamically detect if first_segment is a directory by checking the generated output directory
        let is_directory_prefix =
            if let Some(output_dir) = self.current_output_file.parent().and_then(|p| p.parent()) {
                // Check if a directory exists in the output root for this module name
                let potential_dir = output_dir.join(first_segment);
                potential_dir.is_dir()
            } else {
                false
            };

        // Handle super::super::math::vec3::Vec3 -> super::Vec3
        // This handles cases where Windjammer source uses "use super.super.math.vec3::Vec3"
        if rust_path.starts_with("super::super::") {
            // Extract the path after super::super::
            if let Some(rest_path) = rust_path.strip_prefix("super::super::") {
                // Find the actual type name (last segment)
                let segments: Vec<&str> = rest_path.split("::").collect();
                if let Some(type_name) = segments.last() {
                    if type_name.chars().next().is_some_and(|c| c.is_uppercase()) {
                        // It's a type, use just super::TypeName
                        return format!("use super::{};\n", type_name);
                    }
                }
            }
        }

        // TDD FIX: Detect sibling modules dynamically by checking file existence
        // If we're in a subdirectory and the import doesn't have a known prefix (std::, crate::, super::),
        // check if it's a sibling module file that needs super:: prefix
        let is_sibling_module_file = if is_in_subdirectory {
            // Check if a .wj or .rs file exists for this module in the same directory
            if let Some(parent_dir) = self.current_output_file.parent() {
                let potential_wj_file = parent_dir.join(format!("{}.wj", first_segment));
                let potential_rs_file = parent_dir.join(format!("{}.rs", first_segment));
                let potential_subdir = parent_dir.join(first_segment);

                // If the file/directory exists, it's a sibling module
                potential_wj_file.exists()
                    || potential_rs_file.exists()
                    || potential_subdir.is_dir()
            } else {
                false
            }
        } else {
            false
        };

        let is_actual_module_file = if is_sibling_module_file {
            // Sibling module file exists - use super::
            !is_directory_prefix && first_segment != "super" && first_segment != "self"
        } else {
            // Not a sibling module file - use the old hardcoded list for backwards compatibility
            common_sibling_modules.contains(&first_segment) && !is_directory_prefix
        };

        let _is_sibling_module =
            is_directory_prefix || is_actual_module_file || first_segment == "super";

        // Calculate import prefix for nested output structures
        // When is_module is true, we're generating reusable modules that may be nested
        // In that case, use relative imports based on detected nesting
        let import_prefix = if self.is_module {
            if let Some(nesting_level) = self.get_import_prefix_for_nested_output() {
                // In nested output (e.g., src/generated/core/commands/)
                // Use super:: to navigate up to the root of the generated module
                "super::".repeat(nesting_level)
            } else {
                // Module mode but flat structure - still use crate::
                "crate::".to_string()
            }
        } else {
            // Not in module mode - use crate:: as before
            "crate::".to_string()
        };

        if let Some(alias_name) = alias {
            if is_directory_prefix {
                // THE WINDJAMMER WAY: Use calculated prefix for cross-directory imports
                // math::Vec2 as V -> use super::super::math::Vec2 as V; (in nested output)
                // or use crate::math::Vec2 as V; (in flat output)
                format!("use {}{} as {};\n", import_prefix, rust_path, alias_name)
            } else if is_actual_module_file {
                // Keep module path for actual module files: texture_atlas::TextureAtlas as TA -> use super::texture_atlas::TextureAtlas as TA;
                format!("use super::{} as {};\n", rust_path, alias_name)
            } else {
                format!("use {} as {};\n", rust_path, alias_name)
            }
        } else {
            // Check if already a glob import (ends with ::*)
            if rust_path.ends_with("::*") {
                format!("use {};\n", rust_path)
            } else if is_directory_prefix {
                // THE WINDJAMMER WAY: Use calculated prefix for cross-directory imports
                // math::Vec2 -> use super::super::math::Vec2; (in nested output)
                // or use crate::math::Vec2; (in flat output)
                format!("use {}{};\n", import_prefix, rust_path)
            } else if is_actual_module_file {
                // Keep full path for actual module files to avoid ambiguity
                // texture_atlas::TextureAtlas -> use super::texture_atlas::TextureAtlas;
                format!("use super::{};\n", rust_path)
            } else {
                // Check for crate:: prefix FIRST (before checking if it's a type)
                // This ensures crate::scene::Vec3 gets rewritten to super::super::scene::Vec3
                if rust_path.starts_with("crate::") {
                    // For crate::module imports, rewrite based on nesting
                    // In nested output (e.g., src/generated/core/commands/),
                    // crate::scene::Vec3 should become super::super::scene::Vec3
                    let path_without_crate = rust_path.strip_prefix("crate::").unwrap();
                    format!("use {}{};\n", import_prefix, path_without_crate)
                } else if rust_path.chars().next().is_some_and(|c| c.is_uppercase()) {
                    // Path starts with uppercase (e.g., Vec3, String) - likely a re-exported type
                    // Don't add ::*
                    format!("use {};\n", rust_path)
                } else {
                    // Check if the last segment looks like a type (starts with uppercase)
                    let last_segment = rust_path.split("::").last().unwrap_or("");
                    if last_segment
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_uppercase())
                    {
                        // TDD FIX: For bare module imports (math::Vec3), convert to crate:: prefix
                        // This ensures cross-module imports are absolute, not relative
                        // THE WINDJAMMER WAY: Default to absolute paths for clarity
                        //
                        // But we need to distinguish between:
                        // - Internal modules (math, physics, rendering) -> add crate:: prefix
                        // - External crates (serde, tokio, some_external_crate) -> keep as-is
                        //
                        // Heuristic: Check if first segment matches common internal module names
                        if rust_path.contains("::") {
                            let common_internal_modules = [
                                "math",
                                "physics",
                                "rendering",
                                "world",
                                "game",
                                "audio",
                                "input",
                                "rpg",
                                "ui",
                                "editor",
                                "scene",
                                "collision2d",
                                "networking",
                                "effects",
                                "animation",
                                "ai",
                                "dialogue",
                                "inventory",
                                "quest",
                                "combat",
                                "lighting",
                                "camera",
                                "particles",
                                "terrain",
                                "weather",
                                "save",
                                "config",
                                "debug",
                                "utils",
                                "helpers",
                                "core",
                                "common",
                                "types",
                                "components",
                                "systems",
                                "resources",
                                "entities",
                                "events",
                                "state",
                                "assets",
                                "data",
                                "models",
                                "controllers",
                                "views",
                                "managers",
                                "services",
                                "handlers",
                                "processors",
                            ];

                            let is_likely_internal =
                                common_internal_modules.contains(&first_segment);

                            if is_likely_internal {
                                // Internal module - add crate:: prefix
                                format!("use crate::{};\n", rust_path)
                            } else {
                                // TDD FIX: For external crates and crate-level re-exports, use crate:: prefix
                                // This handles imports like "windjammer_game_core::math::Vec3" where
                                // windjammer_game_core is re-exported at crate root with:
                                //   pub use windjammer_app as windjammer_game_core;
                                // Submodules access it via crate::windjammer_game_core, not self::
                                format!("use crate::{};\n", rust_path)
                            }
                        } else {
                            // Single identifier (Vec3) - likely a type, keep as-is
                            format!("use {};\n", rust_path)
                        }
                    } else {
                        // Likely a module, add ::*
                        format!("use {}::*;\n", rust_path)
                    }
                }
            }
        }
    }
}
