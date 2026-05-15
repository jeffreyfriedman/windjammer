//! Automatic `use super::...` imports for types referenced from sibling `.wj` modules.

use crate::codegen::rust::generator::CodeGenerator;
use crate::parser::Program;

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

        let mut out = String::from("#[allow(unused_imports)]\n");
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
            out.push_str(&format!("use {};\n", rust_path));
        }
        out
    }
}
