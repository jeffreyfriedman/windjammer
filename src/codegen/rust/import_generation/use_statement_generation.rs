//! Main `generate_use` dispatcher and non-stdlib path rules.

use super::external_imports;
use crate::codegen::rust::CodeGenerator;

impl CodeGenerator<'_> {
    pub(in crate::codegen::rust) fn generate_use(
        &self,
        path: &[String],
        alias: Option<&str>,
        is_pub: bool,
    ) -> String {
        let pub_prefix = if is_pub { "pub " } else { "" };
        
        if path.is_empty() {
            return String::new();
        }

        let full_path = path.join(".");

        // SPECIAL CASE: crate::super:: is invalid in Rust - super must be at path start
        // Windjammer "use super::enemy::Enemy" may be parsed as crate.super.enemy.Enemy
        let normalized_for_super = full_path.replace('.', "::");
        if normalized_for_super.starts_with("crate::super::") {
            let path_without_crate = normalized_for_super.strip_prefix("crate::").unwrap();
            return format!("{}use {};\n", pub_prefix, path_without_crate);
        }

        // mod.rs: `crate::<this_module>::X` means the current directory module, not the crate root.
        // Emit `self::` so Rust resolves child modules (fixes E0432).
        if self.is_output_mod_rs() {
            let norm = full_path.replace('.', "::");
            if norm.starts_with("crate::") && !norm.starts_with("crate::super::") {
                if let Some(rest) = norm.strip_prefix("crate::") {
                    if let Some(mod_dir) = self
                        .current_output_file
                        .parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                    {
                        let first_seg = rest.split("::").next().unwrap_or("");
                        if first_seg == mod_dir {
                            if let Some(alias_name) = alias {
                                return format!("{}use self::{} as {};\n", pub_prefix, rest, alias_name);
                            }
                            return format!("{}use self::{};\n", pub_prefix, rest);
                        }
                    }
                }
            }
        }

        // SPECIAL CASE: Handle crate:: imports when in nested module output
        // Examples:
        // - use crate::scene::{A, B} -> use crate::generated::scene::{A, B}
        // - use crate::scene::Scene -> use crate::generated::scene::Scene
        // This applies to both braced and non-braced imports
        if full_path.starts_with("crate::") || full_path.starts_with("crate.") {
            let normalized = full_path.replace('.', "::");

            // Braced crate imports: expand each type to its defining submodule (fixes E0432).
            if normalized.contains("::{") && normalized.contains('}') {
                let braced = self.expand_braced_crate_import(&normalized, alias, is_pub);
                if !braced.is_empty() {
                    return braced;
                }
            }

            // Find the module root (e.g., "generated", "build", "out")
            let module_root = if self.is_module {
                self.get_module_root_name()
            } else {
                None
            };

            let rewritten = if let Some(root_name) = module_root {
                // Rewrite: crate::scene::X -> crate::generated::scene::X
                let path_without_crate = normalized.strip_prefix("crate::").unwrap();
                format!("crate::{}::{}", root_name, path_without_crate)
            } else {
                normalized
            };

            let expanded = self.expand_crate_path_string(&rewritten);

            // TDD FIX: Preserve alias for crate:: imports
            if let Some(alias_name) = alias {
                return format!("{}use {} as {};\n", pub_prefix, expanded, alias_name);
            }
            return format!("{}use {};\n", pub_prefix, expanded);
        }

        // Handle stdlib imports FIRST (before glob handling)
        // This ensures std::ui::*, std::fs::*, etc. are properly skipped
        if let Some(out) = self.try_generate_std_import_use(&full_path, alias) {
            return out;
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
            return format!("{}use {}::*;\n", pub_prefix, rust_path);
        }

        // Handle braced imports: module::{A, B, C} or module.{A, B, C}
        if (full_path.contains("::{") || full_path.contains(".{")) && full_path.contains('}') {
            // Try :: separator first, then . separator
            if let Some((base, items)) = full_path.split_once("::{") {
                return format!("{}use {}::{{{};\n", pub_prefix, base, items);
            } else if let Some((base, items)) = full_path.split_once(".{") {
                let rust_base = base.replace('.', "::");
                return format!("{}use {}::{{{};\n", pub_prefix, rust_base, items);
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
                        return format!("{}use crate::{};\n", pub_prefix, rust_path);
                    }
                }
                // For crate::module imports, just import the module (not ::*)
                // This allows qualified usage like module::func() in the code
                return format!("{}use crate::{};\n", pub_prefix, rust_path);
            } else {
                // Module import: ./config
                // In the main entry point (is_module=false), modules are already in scope via pub mod declarations
                // In submodules (is_module=true), we need to explicitly use sibling modules
                let module_name = stripped.split('/').next_back().unwrap_or(stripped);
                if let Some(alias_name) = alias {
                    return format!("{}use crate::{} as {};\n", pub_prefix, module_name, alias_name);
                } else if self.is_module {
                    // In a module, we need to explicitly use sibling modules
                    return format!("{}use crate::{};\n", pub_prefix, module_name);
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

        // TDD FIX: Paths starting with super:: must NOT get crate:: prepended
        // Rust requires super at path start: "use super::enemy::Enemy" not "use crate::super::enemy::Enemy"
        if rust_path.starts_with("super::") {
            return format!("{}use {};\n", pub_prefix, rust_path);
        }

        // TDD FIX: Bare paths referencing inline modules need self:: prefix.
        // In Rust, `pub use inner_module::MyType;` is E0432 when `inner_module`
        // is declared as `mod inner_module { ... }` in the same file.
        // Must be `pub use self::inner_module::MyType;` instead.
        if let Some(first_seg) = rust_path.split("::").next() {
            if self.inline_module_names.contains(first_seg) {
                if let Some(alias_name) = alias {
                    return format!("{}use self::{} as {};\n", pub_prefix, rust_path, alias_name);
                }
                return format!("{}use self::{};\n", pub_prefix, rust_path);
            }
        }

        // TDD FIX: Handle imports from sibling modules (Part 2 - Nested Import Bug)
        // When in a subdirectory (e.g., rendering/sprite.wj) and importing a sibling (texture::Texture),
        // we need to detect this and rewrite to super::texture::Texture
        //
        // Detection strategy:
        // 1. Check if we're in a subdirectory - check the INPUT file (.wj), not the output (.rs)
        //    because output might be flat (build/mod.rs) while input is nested (achievement/mod.wj)
        // 2. Check if the import is bare (no std::, crate::, super:: prefix)
        // 3. Assume it's a sibling module and use super:: prefix
        //
        // THE WINDJAMMER WAY: Smart defaults that work 99% of the time
        // TDD COMPILER FIX: Check INPUT file structure, not OUTPUT structure!
        // Output might be flat (build/*.rs) while input is nested (src/achievement/mod.wj)
        let is_in_subdirectory = self
            .current_wj_file
            .parent()
            .and_then(|p| p.file_name())
            .is_some(); // If parent has a name, we're in a subdirectory

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
                    if self.is_output_mod_rs() {
                        // mod.wj: `pub use animation::Animation` → self::animation::Animation (not super::Animation)
                        if let Some(alias_name) = alias {
                            return format!("{}use self::{} as {};\n", pub_prefix, rust_path, alias_name);
                        }
                        return format!("{}use self::{};\n", pub_prefix, rust_path);
                    }
                    // Strip the parent directory name and use super:: instead
                    let path_without_parent = rust_path
                        .strip_prefix(&format!("{}::", parent_dir))
                        .unwrap();
                    if let Some(alias_name) = alias {
                        return format!("{}use super::{} as {};\n", pub_prefix, path_without_parent, alias_name);
                    }
                    return format!("{}use super::{};\n", pub_prefix, path_without_parent);
                }
            }
        }

        // BUGFIX: Handle imports from sibling modules (flat directory structure)
        let common_sibling_modules = ["vec2", "vec3", "vec4", "mat4", "quat", "color"];

        // Extract first segment early so we can use it in multiple places
        let first_segment = rust_path.split("::").next().unwrap_or("");

        // TDD FIX: Dynamically detect if first_segment is a directory by checking the generated output directory
        let is_child_of_mod_rs = self.is_child_module_of_mod_rs_dir(first_segment);
        let is_directory_prefix = !is_child_of_mod_rs
            && if let Some(output_dir) = self.current_output_file.parent().and_then(|p| p.parent())
            {
                let potential_dir = output_dir.join(first_segment);
                potential_dir.is_dir()
            } else {
                false
            };

        // Handle super::super::math::vec3::Vec3 -> super::Vec3
        if rust_path.starts_with("super::super::") {
            // Extract the path after super::super::
            if let Some(rest_path) = rust_path.strip_prefix("super::super::") {
                // Find the actual type name (last segment)
                let segments: Vec<&str> = rest_path.split("::").collect();
                if let Some(type_name) = segments.last() {
                    if type_name.chars().next().is_some_and(|c| c.is_uppercase()) {
                        // It's a type, use just super::TypeName
                        return format!("{}use super::{};\n", pub_prefix, type_name);
                    }
                }
            }
        }

        // TDD FIX: Detect sibling modules dynamically by checking file existence
        let is_sibling_module_file = if is_in_subdirectory {
            if let Some(parent_dir) = self.current_wj_file.parent() {
                let potential_wj_file = parent_dir.join(format!("{}.wj", first_segment));
                let potential_rs_file = parent_dir.join(format!("{}.rs", first_segment));
                let potential_subdir = parent_dir.join(first_segment);

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
            !is_directory_prefix && first_segment != "super" && first_segment != "self"
        } else {
            common_sibling_modules.contains(&first_segment) && !is_directory_prefix
        };

        let _is_sibling_module =
            is_directory_prefix || is_actual_module_file || first_segment == "super";

        // Calculate import prefix for nested output structures
        let import_prefix = if self.is_module {
            if let Some(nesting_level) = self.get_import_prefix_for_nested_output() {
                "super::".repeat(nesting_level)
            } else {
                "crate::".to_string()
            }
        } else {
            "crate::".to_string()
        };

        if let Some(alias_name) = alias {
            if is_directory_prefix {
                let rp = self.expand_bare_module_path_for_type(&rust_path);
                format!("{}use {}{} as {};\n", pub_prefix, import_prefix, rp, alias_name)
            } else if is_actual_module_file {
                if self.is_output_mod_rs() {
                    format!("{}use self::{} as {};\n", pub_prefix, rust_path, alias_name)
                } else {
                    format!("{}use super::{} as {};\n", pub_prefix, rust_path, alias_name)
                }
            } else {
                format!("{}use {} as {};\n", pub_prefix, rust_path, alias_name)
            }
        } else if rust_path.ends_with("::*") {
            format!("{}use {};\n", pub_prefix, rust_path)
        } else if is_directory_prefix {
            let rp = self.expand_bare_module_path_for_type(&rust_path);
            format!("{}use {}{};\n", pub_prefix, import_prefix, rp)
        } else if is_actual_module_file {
            if self.is_output_mod_rs() {
                format!("{}use self::{};\n", pub_prefix, rust_path)
            } else {
                format!("{}use super::{};\n", pub_prefix, rust_path)
            }
        } else if rust_path.starts_with("crate::") {
            let path_without_crate = rust_path.strip_prefix("crate::").unwrap();
            let pwc = self.expand_bare_module_path_for_type(path_without_crate);
            format!("{}use {}{};\n", pub_prefix, import_prefix, pwc)
        } else if rust_path.chars().next().is_some_and(|c| c.is_uppercase()) {
            format!("{}use {};\n", pub_prefix, rust_path)
        } else {
            let last_segment = rust_path.split("::").last().unwrap_or("");
            if last_segment
                .chars()
                .next()
                .is_some_and(|c| c.is_uppercase())
            {
                if is_in_subdirectory && is_sibling_module_file {
                    if self.is_output_mod_rs() {
                        format!("{}use self::{};\n", pub_prefix, rust_path)
                    } else {
                        format!("{}use {};\n", pub_prefix, rust_path)
                    }
                } else if rust_path.contains("::") {
                    let is_likely_internal =
                        external_imports::is_likely_internal_module(first_segment);

                    let rp = self.expand_bare_module_path_for_type(&rust_path);
                    if is_likely_internal {
                        format!("{}use crate::{};\n", pub_prefix, rp)
                    } else {
                        format!("{}use {};\n", pub_prefix, rp)
                    }
                } else {
                    format!("{}use {};\n", pub_prefix, rust_path)
                }
            } else if rust_path.contains("::") {
                let is_likely_internal = external_imports::is_likely_internal_module(first_segment);
                let is_external = external_imports::is_external_crate(first_segment);
                
                if is_likely_internal {
                    format!("{}use crate::{};\n", pub_prefix, rust_path)
                } else if is_external {
                    // External crate module - import the module itself, not glob its contents
                    format!("{}use {};\n", pub_prefix, rust_path)
                } else {
                    format!("{}use {};\n", pub_prefix, rust_path)
                }
            } else {
                format!("{}use {}::*;\n", pub_prefix, rust_path)
            }
        }
    }
}
