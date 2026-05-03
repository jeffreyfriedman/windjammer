//! Import/use statement generation for Rust code.
//!
//! This module handles the conversion of Windjammer import paths to Rust `use` statements.
//! It resolves crate::, std::, relative (./, ../), and sibling module imports, including
//! support for nested output directory structures (e.g., src/generated/core/commands/).

use super::CodeGenerator;

impl CodeGenerator<'_> {
    /// Calculate the import prefix for cross-module imports based on output file nesting
    /// Returns the number of directory levels to go up (for super:: prefixes)
    pub(crate) fn get_import_prefix_for_nested_output(&self) -> Option<usize> {
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

    /// True when the Rust file being generated is a directory module root (`mod.rs`).
    pub(crate) fn is_output_mod_rs(&self) -> bool {
        self.current_output_file
            .file_name()
            .and_then(|n| n.to_str())
            == Some("mod.rs")
    }

    /// True if `first_segment` names a submodule living in the same directory as this `mod.rs`
    /// (matches sibling `.wj` / `.rs` next to `mod.wj` / `mod.rs`, submodule directory, or
    /// `name/mod.wj` under the output dir).
    ///
    /// Multipass emits only `.rs` next to `mod.rs`; checking the **source** directory
    /// (`current_wj_file.parent()`) ensures we still classify children when `mod.wj` is
    /// compiled before sibling `.rs` exists, and avoids false `is_directory_prefix` when a
    /// **top-level** directory shares the submodule name (E0432).
    fn is_child_module_of_mod_rs_dir(&self, first_segment: &str) -> bool {
        if !self.is_output_mod_rs() {
            return false;
        }
        let Some(mod_dir) = self.current_output_file.parent() else {
            return false;
        };
        let child = mod_dir.join(first_segment);
        if child.is_dir() {
            return true;
        }
        let out_wj = mod_dir.join(format!("{}.wj", first_segment));
        if out_wj.exists() {
            return true;
        }
        let out_rs = mod_dir.join(format!("{}.rs", first_segment));
        if out_rs.exists() {
            return true;
        }
        if let Some(wj_parent) = self.current_wj_file.parent() {
            if wj_parent.join(format!("{}.wj", first_segment)).exists() {
                return true;
            }
            if wj_parent.join(format!("{}.rs", first_segment)).exists() {
                return true;
            }
        }
        false
    }

    /// When the same type name exists in multiple modules, pick the defining path that extends
    /// the import's parent prefix (`crate::autotile::TileId` → parent `[autotile]`).
    fn select_def_mod_for_import_prefix<'a>(
        candidates: &'a [Vec<String>],
        logical_parent: &[String],
    ) -> Option<&'a Vec<String>> {
        let matches: Vec<&Vec<String>> = candidates
            .iter()
            .filter(|c| {
                c.len() > logical_parent.len() && c[..logical_parent.len()] == logical_parent[..]
            })
            .collect();
        if matches.is_empty() {
            return None;
        }
        if matches.len() == 1 {
            return Some(matches[0]);
        }
        // Prefer the shortest suffix after `logical_parent` so duplicate type names resolve to the
        // canonical submodule (e.g. `input::input::Input` over `input::input_interface::Input`).
        Some(
            *matches
                .iter()
                .min_by_key(|c| {
                    let tail = &c[logical_parent.len()..];
                    (tail.len(), tail.iter().map(|s| s.len()).sum::<usize>())
                })
                .expect("matches non-empty"),
        )
    }

    fn split_leading_output_root(&self, segs: &[String]) -> (Option<String>, Vec<String>) {
        if let Some(root) = self.get_module_root_name() {
            if segs.first().map(|s| s.as_str()) == Some(root.as_str()) {
                return (Some(root), segs[1..].to_vec());
            }
        }
        (None, segs.to_vec())
    }

    /// Segments after `crate::` (and after optional output-root), ending with a type name.
    fn expand_type_path_after_crate_root(&self, body: &[String]) -> Vec<String> {
        if body.len() < 2 || self.type_defining_modules.is_empty() {
            return body.to_vec();
        }
        let type_name = &body[body.len() - 1];
        if !type_name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
        {
            return body.to_vec();
        }
        let logical_parent = &body[..body.len() - 1];
        let candidates = match self.type_defining_modules.get(type_name) {
            Some(c) if !c.is_empty() => c.as_slice(),
            _ => return body.to_vec(),
        };
        let Some(def_mod) = Self::select_def_mod_for_import_prefix(candidates, logical_parent)
        else {
            return body.to_vec();
        };
        if *def_mod == *logical_parent {
            return body.to_vec();
        }
        if def_mod.len() > logical_parent.len()
            && def_mod[..logical_parent.len()] == logical_parent[..]
        {
            let mut out = def_mod.clone();
            out.push(type_name.clone());
            return out;
        }
        body.to_vec()
    }

    /// Expand `math::Vec3`-style paths (no `crate::` prefix) for nested output / internal imports.
    fn expand_bare_module_path_for_type(&self, rust_path: &str) -> String {
        if self.type_defining_modules.is_empty() {
            return rust_path.to_string();
        }
        let segs: Vec<String> = rust_path.split("::").map(String::from).collect();
        self.expand_type_path_after_crate_root(&segs).join("::")
    }

    /// Rewrite `crate::parent::Type` → `crate::parent::submodule::Type` using multipass `type_defining_modules`.
    pub(crate) fn expand_crate_path_string(&self, rust_path: &str) -> String {
        if self.type_defining_modules.is_empty() {
            return rust_path.to_string();
        }
        let trimmed = rust_path.trim();
        if !trimmed.starts_with("crate::") {
            return rust_path.to_string();
        }
        let rest = trimmed.strip_prefix("crate::").unwrap();
        let segs: Vec<String> = rest
            .split("::")
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        if segs.is_empty() {
            return rust_path.to_string();
        }
        let (root_opt, body) = self.split_leading_output_root(&segs);
        let expanded_body = self.expand_type_path_after_crate_root(&body);
        if expanded_body == body {
            return rust_path.to_string();
        }
        let mut out = String::from("crate::");
        if let Some(root) = root_opt {
            out.push_str(&root);
            out.push_str("::");
        }
        out.push_str(&expanded_body.join("::"));
        out
    }

    fn expand_braced_crate_import(&self, normalized: &str, alias: Option<&str>) -> String {
        let module_root = if self.is_module {
            self.get_module_root_name()
        } else {
            None
        };
        let with_root = if let Some(ref root_name) = module_root {
            if normalized.starts_with("crate::") {
                let w = normalized.strip_prefix("crate::").unwrap();
                format!("crate::{}::{}", root_name, w)
            } else {
                normalized.to_string()
            }
        } else {
            normalized.to_string()
        };

        let Some((base, rest)) = with_root.split_once("::{") else {
            return if let Some(a) = alias {
                format!("use {} as {};\n", with_root, a)
            } else {
                format!("use {};\n", with_root)
            };
        };
        let Some(inner) = rest.strip_suffix('}') else {
            return if let Some(a) = alias {
                format!("use {} as {};\n", with_root, a)
            } else {
                format!("use {};\n", with_root)
            };
        };
        let types: Vec<&str> = inner
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if types.is_empty() {
            return if let Some(a) = alias {
                format!("use {} as {};\n", with_root, a)
            } else {
                format!("use {};\n", with_root)
            };
        }

        let all_pascal = types
            .iter()
            .all(|t| t.chars().next().is_some_and(|c| c.is_ascii_uppercase()));
        if !all_pascal {
            return if let Some(a) = alias {
                format!("use {} as {};\n", with_root, a)
            } else {
                format!("use {};\n", with_root)
            };
        }

        if types.len() == 1 {
            let single = format!("{}::{}", base, types[0]);
            let exp = self.expand_crate_path_string(&single);
            return if let Some(a) = alias {
                format!("use {} as {};\n", exp, a)
            } else {
                format!("use {};\n", exp)
            };
        }

        // Multiple types may live in different submodules — emit one `use` per type.
        let mut lines = String::new();
        for typ in types {
            let single = format!("{}::{}", base, typ);
            let exp = self.expand_crate_path_string(&single);
            lines.push_str(&format!("use {};\n", exp));
        }
        lines
    }

    pub(super) fn generate_use(&self, path: &[String], alias: Option<&str>) -> String {
        if path.is_empty() {
            return String::new();
        }

        let full_path = path.join(".");

        // SPECIAL CASE: crate::super:: is invalid in Rust - super must be at path start
        // Windjammer "use super::enemy::Enemy" may be parsed as crate.super.enemy.Enemy
        let normalized_for_super = full_path.replace('.', "::");
        if normalized_for_super.starts_with("crate::super::") {
            let path_without_crate = normalized_for_super.strip_prefix("crate::").unwrap();
            return format!("use {};\n", path_without_crate);
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
                                return format!("use self::{} as {};\n", rest, alias_name);
                            }
                            return format!("use self::{};\n", rest);
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
                let braced = self.expand_braced_crate_import(&normalized, alias);
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
                return format!("use {} as {};\n", expanded, alias_name);
            }
            return format!("use {};\n", expanded);
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

            // Rust std modules: pass through as `use std::fs`, `use std::env`, etc.
            if module_base == "fs"
                || module_base.starts_with("fs::")
                || module_base == "process"
                || module_base.starts_with("process::")
                || module_base == "env"
                || module_base.starts_with("env::")
            {
                return format!("use std::{};\n", module_name);
            }

            // Platform APIs with no Rust std equivalent - skip
            if module_base == "dialog"
                || module_base.starts_with("dialog::")
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

        // TDD FIX: Paths starting with super:: must NOT get crate:: prepended
        // Rust requires super at path start: "use super::enemy::Enemy" not "use crate::super::enemy::Enemy"
        if rust_path.starts_with("super::") {
            return format!("use {};\n", rust_path);
        }

        // TDD FIX: Bare paths referencing inline modules need self:: prefix.
        // In Rust, `pub use inner_module::MyType;` is E0432 when `inner_module`
        // is declared as `mod inner_module { ... }` in the same file.
        // Must be `pub use self::inner_module::MyType;` instead.
        if let Some(first_seg) = rust_path.split("::").next() {
            if self.inline_module_names.contains(first_seg) {
                if let Some(alias_name) = alias {
                    return format!("use self::{} as {};\n", rust_path, alias_name);
                }
                return format!("use self::{};\n", rust_path);
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
        // Output might be flat (build/*.rs) while input is nested (src_wj/achievement/mod.wj)
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
                            return format!("use self::{} as {};\n", rust_path, alias_name);
                        }
                        return format!("use self::{};\n", rust_path);
                    }
                    // Strip the parent directory name and use super:: instead
                    let path_without_parent = rust_path
                        .strip_prefix(&format!("{}::", parent_dir))
                        .unwrap();
                    if let Some(alias_name) = alias {
                        return format!("use super::{} as {};\n", path_without_parent, alias_name);
                    }
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
        // When generating mod.rs, prefer a local child module (./state_machine) over a same-named top-level
        // folder under the output root; otherwise we wrongly emit super::state_machine::... (E0432).
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
        // TDD COMPILER FIX: Check INPUT file directory, not OUTPUT directory!
        let is_sibling_module_file = if is_in_subdirectory {
            // Check if a .wj or .rs file exists for this module in the INPUT directory
            if let Some(parent_dir) = self.current_wj_file.parent() {
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
                let rp = self.expand_bare_module_path_for_type(&rust_path);
                format!("use {}{} as {};\n", import_prefix, rp, alias_name)
            } else if is_actual_module_file {
                if self.is_output_mod_rs() {
                    format!("use self::{} as {};\n", rust_path, alias_name)
                } else {
                    // Keep module path for actual module files: texture_atlas::TextureAtlas as TA -> use super::texture_atlas::TextureAtlas as TA;
                    format!("use super::{} as {};\n", rust_path, alias_name)
                }
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
                let rp = self.expand_bare_module_path_for_type(&rust_path);
                format!("use {}{};\n", import_prefix, rp)
            } else if is_actual_module_file {
                // TDD COMPILER FIX: If the first segment matches a submodule declared in the SAME file,
                // don't add ANY prefix (not super::, not crate::).
                // In mod.rs files, "pub mod foo;" declares foo as a submodule accessible without prefix.
                //
                // Example in achievement/mod.rs:
                //   pub mod achievement_id;
                //   pub use achievement_id::AchievementId;  // ✅ CORRECT (no prefix)
                //   NOT: pub use super::achievement_id::AchievementId;  // ❌ super is PARENT module
                //
                // Only use super:: for imports from sibling FILES, not submodules in same file.
                if self.is_output_mod_rs() {
                    format!("use self::{};\n", rust_path)
                } else {
                    // Match the aliased branch above: sibling module paths need `super::`
                    // so Rust resolves them from the parent module (bare `foo::` is E0432).
                    format!("use super::{};\n", rust_path)
                }
            } else {
                // Check for crate:: prefix FIRST (before checking if it's a type)
                // This ensures crate::scene::Vec3 gets rewritten to super::super::scene::Vec3
                if rust_path.starts_with("crate::") {
                    // For crate::module imports, rewrite based on nesting
                    // In nested output (e.g., src/generated/core/commands/),
                    // crate::scene::Vec3 should become super::super::scene::Vec3
                    let path_without_crate = rust_path.strip_prefix("crate::").unwrap();
                    let pwc = self.expand_bare_module_path_for_type(path_without_crate);
                    format!("use {}{};\n", import_prefix, pwc)
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
                        // TDD COMPILER FIX: pub use in mod.wj should preserve relative paths for submodules
                        // When in achievement/mod.wj doing "pub use achievement_id::AchievementId",
                        // achievement_id is a SUBMODULE (declared in same file), so keep it relative!
                        //
                        // Check if this is likely a relative import of a submodule:
                        // - We're in a directory (is_in_subdirectory)
                        // - First segment is a sibling module file (is_sibling_module_file)
                        if is_in_subdirectory && is_sibling_module_file {
                            if self.is_output_mod_rs() {
                                format!("use self::{};\n", rust_path)
                            } else {
                                // Keep relative - this is re-exporting a local submodule
                                format!("use {};\n", rust_path)
                            }
                        } else {
                            // Otherwise, add crate:: prefix for absolute import
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

                                let rp = self.expand_bare_module_path_for_type(&rust_path);
                                if is_likely_internal {
                                    format!("use crate::{};\n", rp)
                                } else {
                                    // External crate or unknown module — do NOT prepend crate::
                                    // External crate imports (e.g. windjammer_game_core::math::Vec3)
                                    // must remain as-is so Cargo resolves them from [dependencies].
                                    format!("use {};\n", rp)
                                }
                            } else {
                                // Single identifier (Vec3) - likely a type, keep as-is
                                format!("use {};\n", rust_path)
                            }
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
