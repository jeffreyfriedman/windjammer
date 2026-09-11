//! Automatic `use super::...` imports for types referenced from sibling `.wj` modules.

use crate::analyzer::SignatureRegistry;
use crate::codegen::rust::generator::CodeGenerator;
use crate::codegen::rust::stdlib_method_traits::runtime_std_module_for_type;
use crate::parser::{Item, Program};

impl<'ast> CodeGenerator<'ast> {
    /// Generate `use super::...` lines for types referenced in this file but defined elsewhere.
    pub(crate) fn format_auto_super_type_imports(&self, program: &Program<'ast>) -> String {
        if !self.is_module {
            return String::new();
        }
        let paths = crate::analyzer::type_collector::auto_super_type_import_paths(program);
        if paths.is_empty() {
            return String::new();
        }

        let current_module = self.library_source_root.as_ref().and_then(|base| {
            crate::analyzer::type_collector::wj_file_to_module_path(base, &self.current_wj_file)
        });

        let mut uses = String::new();
        for path in paths {
            let (_, type_name) = crate::analyzer::type_collector::split_qualified_type_path(&path);
            let key = if type_name.is_empty() {
                path.as_str()
            } else {
                type_name
            };

            let resolved = if let Some(ref cur) = current_module {
                if !self.type_defining_modules.is_empty() {
                    self.type_defining_modules.get(key).and_then(|candidates| {
                        if candidates.is_empty() {
                            return None;
                        }
                        let best_lcp = candidates
                            .iter()
                            .map(|def_mod| {
                                crate::analyzer::type_collector::longest_common_prefix_len(
                                    cur, def_mod,
                                )
                            })
                            .max()?;
                        let tied: Vec<&Vec<String>> = candidates
                            .iter()
                            .filter(|def_mod| {
                                crate::analyzer::type_collector::longest_common_prefix_len(
                                    cur, def_mod,
                                ) == best_lcp
                            })
                            .collect();
                        let best = tied.iter().min_by_key(|def_mod| {
                            let tail = &def_mod[best_lcp..];
                            (tail.len(), tail.iter().map(|s| s.len()).sum::<usize>())
                        })?;
                        crate::analyzer::type_collector::rust_use_path_from_module_to_type(
                            cur, best, key,
                        )
                    })
                } else {
                    None
                }
            } else {
                None
            };

            // Crate-local types win. Stdlib runtime types must not fall through to
            // `use super::Type` (nested `use std::http::*` → E0432).
            if resolved.is_none() {
                if let Some(module) = runtime_std_module_for_type(key) {
                    if std_use_covers_unqualified_type(program, module, key) {
                        continue;
                    }
                    if let Some(fq) = SignatureRegistry::stdlib().runtime_rust_path_for_type(key) {
                        uses.push_str(&format!("use {fq};\n"));
                        continue;
                    }
                }
                if let Some(fq) = self.stdlib_type_rust_paths.get(key) {
                    uses.push_str(&format!("use {fq};\n"));
                    continue;
                }
            }

            // `rust_use_path_from_module_to_type` already emits the correct `super::` depth for the
            // Rust module tree; do not prepend filesystem nesting again (would double `super::`).
            let rust_path = if let Some(r) = resolved {
                r
            } else {
                let p = path.replace('.', "::");
                let chain = self
                    .get_import_prefix_for_nested_output()
                    .map(|n| "super::".repeat(n))
                    .unwrap_or_else(|| "super::".to_string());
                format!("{}{}", chain, p)
            };
            uses.push_str(&format!("use {};\n", rust_path));
        }
        if uses.is_empty() {
            return String::new();
        }
        format!("#[allow(unused_imports)]\n{uses}")
    }

    /// Import custom element types referenced only in generated Rust type ascriptions
    /// (e.g. `let jobs: Vec<JobRecord> = loaded.1`) when the AST did not `use` them.
    pub(crate) fn format_body_inferred_custom_type_imports(
        &self,
        body: &str,
        existing_imports: &str,
        program: &Program<'ast>,
    ) -> String {
        if !self.is_module || self.type_defining_modules.is_empty() {
            return String::new();
        }
        let local_types = crate::analyzer::type_collector::collect_local_type_names(program);
        let current_module = self.library_source_root.as_ref().and_then(|base| {
            crate::analyzer::type_collector::wj_file_to_module_path(base, &self.current_wj_file)
        });
        let mut names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for token in extract_custom_types_from_rust_type_annotations(body) {
            if local_types.contains(&token) {
                continue;
            }
            if crate::type_classification::is_prelude_or_primitive(&token) {
                continue;
            }
            if existing_imports.contains(&format!("::{token}"))
                || existing_imports.contains(&format!("{token};"))
                || existing_imports.contains(&format!(" {token};"))
            {
                continue;
            }
            names.insert(token);
        }
        if names.is_empty() {
            return String::new();
        }
        let mut uses = String::new();
        for type_name in names {
            let resolved = if let Some(ref cur) = current_module {
                self.type_defining_modules.get(&type_name).and_then(|candidates| {
                    if candidates.is_empty() {
                        return None;
                    }
                    let best_lcp = candidates
                        .iter()
                        .map(|def_mod| {
                            crate::analyzer::type_collector::longest_common_prefix_len(cur, def_mod)
                        })
                        .max()?;
                    let tied: Vec<&Vec<String>> = candidates
                        .iter()
                        .filter(|def_mod| {
                            crate::analyzer::type_collector::longest_common_prefix_len(
                                cur, def_mod,
                            ) == best_lcp
                        })
                        .collect();
                    let best = tied.iter().min_by_key(|def_mod| {
                        let tail = &def_mod[best_lcp..];
                        (tail.len(), tail.iter().map(|s| s.len()).sum::<usize>())
                    })?;
                    crate::analyzer::type_collector::rust_use_path_from_module_to_type(
                        cur, best, &type_name,
                    )
                })
            } else {
                None
            };
            let rust_path = if let Some(r) = resolved {
                r
            } else {
                let expanded = self.expand_crate_path_string(&format!("crate::{type_name}"));
                if expanded == format!("crate::{type_name}") {
                    continue;
                }
                expanded
            };
            uses.push_str(&format!("#[allow(unused_imports)]\nuse {rust_path};\n"));
        }
        uses
    }
}

/// Collect PascalCase type names appearing in generated Rust generic annotations.
///
/// Handles tuple element types inside collections (WDB-118): `Vec<(Key, Value)>`
/// must yield `Key` and `Value`, not the paren-stained tokens `(Key` / `Value)`.
fn extract_custom_types_from_rust_type_annotations(body: &str) -> Vec<String> {
    let mut out = std::collections::BTreeSet::new();
    let wrappers = [
        "Vec<", "Option<", "SmallVec<[", "HashMap<", "HashSet<", "BTreeMap<", "BTreeSet<",
    ];
    for wrapper in wrappers {
        let mut rest = body;
        while let Some(start) = rest.find(wrapper) {
            let after = &rest[start + wrapper.len()..];
            let Some(end) = find_matching_angle_close(after) else {
                break;
            };
            let inner = &after[..end];
            collect_pascal_case_type_idents(inner, &mut out);
            rest = &after[end..];
        }
    }
    out.into_iter().collect()
}

/// Depth-aware `>` matcher so nested generics (`Vec<Option<(A, B)>>`) stay intact.
fn find_matching_angle_close(s: &str) -> Option<usize> {
    let mut depth = 1usize;
    for (i, ch) in s.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn collect_pascal_case_type_idents(s: &str, out: &mut std::collections::BTreeSet<String>) {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_uppercase() {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let name: String = chars[start..i].iter().collect();
            if !crate::type_classification::is_prelude_or_primitive(&name) {
                out.insert(name);
            }
        } else {
            i += 1;
        }
    }
}

#[cfg(test)]
mod extract_annotation_type_tests {
    use super::extract_custom_types_from_rust_type_annotations;

    #[test]
    fn extracts_tuple_element_types_inside_vec() {
        let names = extract_custom_types_from_rust_type_annotations(
            "let pairs: Vec<(Key, Value)> = to_pairs();",
        );
        assert!(names.iter().any(|n| n == "Key"), "got {names:?}");
        assert!(names.iter().any(|n| n == "Value"), "got {names:?}");
    }
}

/// `use std::{module}::*` / `use std::{module}::{Type}` already brings `type_name` into scope.
fn std_use_covers_unqualified_type(program: &Program<'_>, module: &str, type_name: &str) -> bool {
    program.items.iter().any(|item| {
        let Item::Use { path, .. } = item else {
            return false;
        };
        if path.first().map(String::as_str) != Some("std") {
            return false;
        }
        if path.get(1).map(String::as_str) != Some(module) {
            return false;
        }
        match path.get(2).map(String::as_str) {
            Some("*") => true,
            Some(name) if name == type_name => true,
            Some(braced)
                if braced.contains('{')
                    && braced
                        .split(|c| c == '{' || c == '}' || c == ',')
                        .any(|p| p.trim() == type_name) =>
            {
                true
            }
            _ => false,
        }
    })
}
