//! Import/use statement generation for Rust code.
//!
//! This module handles the conversion of Windjammer import paths to Rust `use` statements.
//! It resolves crate::, std::, relative (./, ../), and sibling module imports, including
//! support for nested output directory structures (e.g., src/generated/core/commands/).

mod external_imports;
mod stdlib_imports;
mod use_statement_generation;

use crate::codegen::rust::CodeGenerator;

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
        let name = self
            .current_output_file
            .file_name()
            .and_then(|n| n.to_str());
        name == Some("mod.rs") || name == Some("_mod_items.rs")
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

    fn expand_braced_crate_import(&self, normalized: &str, alias: Option<&str>, is_pub: bool) -> String {
        let pub_prefix = if is_pub { "pub " } else { "" };
        
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
                format!("{}use {} as {};\n", pub_prefix, with_root, a)
            } else {
                format!("{}use {};\n", pub_prefix, with_root)
            };
        };
        let Some(inner) = rest.strip_suffix('}') else {
            return if let Some(a) = alias {
                format!("{}use {} as {};\n", pub_prefix, with_root, a)
            } else {
                format!("{}use {};\n", pub_prefix, with_root)
            };
        };
        let types: Vec<&str> = inner
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if types.is_empty() {
            return if let Some(a) = alias {
                format!("{}use {} as {};\n", pub_prefix, with_root, a)
            } else {
                format!("{}use {};\n", pub_prefix, with_root)
            };
        }

        let all_pascal = types
            .iter()
            .all(|t| t.chars().next().is_some_and(|c| c.is_ascii_uppercase()));
        if !all_pascal {
            return if let Some(a) = alias {
                format!("{}use {} as {};\n", pub_prefix, with_root, a)
            } else {
                format!("{}use {};\n", pub_prefix, with_root)
            };
        }

        if types.len() == 1 {
            let single = format!("{}::{}", base, types[0]);
            let exp = self.expand_crate_path_string(&single);
            return if let Some(a) = alias {
                format!("{}use {} as {};\n", pub_prefix, exp, a)
            } else {
                format!("{}use {};\n", pub_prefix, exp)
            };
        }

        // Multiple types may live in different submodules — emit one `use` per type.
        let mut lines = String::new();
        for typ in types {
            let single = format!("{}::{}", base, typ);
            let exp = self.expand_crate_path_string(&single);
            lines.push_str(&format!("{}use {};\n", pub_prefix, exp));
        }
        lines
    }
}
