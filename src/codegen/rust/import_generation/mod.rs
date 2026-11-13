//! Import/use statement generation for Rust code.
//!
//! This module handles the conversion of Windjammer import paths to Rust `use` statements.
//! It resolves crate::, std::, relative (./, ../), and sibling module imports, including
//! support for nested output directory structures (e.g., src/generated/core/commands/).

mod external_imports;
mod module_position;
mod stdlib_imports;
mod use_statement_generation;

use crate::codegen::rust::CodeGenerator;

impl CodeGenerator<'_> {
    /// Calculate the import prefix for cross-module imports based on output file nesting
    /// Returns the number of directory levels to go up (for super:: prefixes)
    pub(crate) fn get_import_prefix_for_nested_output(&self) -> Option<usize> {
        // `super::` once per directory module between this file and the generated tree's root.
        let depth = self.output_module_dirs()?.len();
        (depth > 0).then_some(depth)
    }

    fn get_module_root_name(&self) -> Option<String> {
        // A host crate that mounts the generated tree (`pub mod generated;`) names it; a tree of its own has none.
        self.host_mount_module()
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
        // The children this directory module declares in the source tree it mirrors.
        let Some(dirs) = self.output_module_dirs() else { return false };
        self.source_module_declares_child(&dirs, first_segment)
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

    /// Resolve a bare multipass import (e.g. `adapters::seed_writer::SeedWriter`) to the correct
    /// Rust module-relative path using the Windjammer module tree — not filesystem nesting depth.
    ///
    /// From `composition/deps.wj` (`composition::deps`) importing `adapters::seed_writer::SeedWriter`
    /// yields `super::super::adapters::seed_writer::SeedWriter`, not `super::adapters::…` (E0433).
    pub(crate) fn rust_use_path_from_current_file(&self, rust_path: &str) -> Option<String> {
        let base = self.library_source_root.as_ref()?;
        let current =
            crate::analyzer::type_collector::wj_file_to_module_path(base, &self.current_wj_file)?;

        let expanded = self.expand_bare_module_path_for_type(rust_path);
        let segs: Vec<String> = expanded.split("::").map(String::from).collect();
        if segs.is_empty() {
            return None;
        }

        let last = segs.last()?;
        let defining_module = &segs[..segs.len() - 1];
        if defining_module.is_empty() {
            return Some(last.clone());
        }

        crate::analyzer::type_collector::rust_use_path_from_module_to_type(
            &current,
            defining_module,
            last,
        )
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

    fn expand_braced_crate_import(
        &self,
        normalized: &str,
        alias: Option<&str>,
        is_pub: bool,
    ) -> String {
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
