//! `std::…` import paths → Rust `use` output (including runtime remapping).

use crate::codegen::rust::CodeGenerator;

impl CodeGenerator<'_> {
    /// If `full_path` is a Windjammer `std::` / `std.` import, returns the generated `use` line(s).
    /// Otherwise returns [`None`] so the caller can continue with other rules.
    pub(in crate::codegen::rust) fn try_generate_std_import_use(
        &self,
        full_path: &str,
        alias: Option<&str>,
    ) -> Option<String> {
        if !(full_path.starts_with("std::") || full_path.starts_with("std.")) {
            return None;
        }

        // Normalize to use :: separator
        let normalized = full_path.replace('.', "::");
        let module_name = normalized.strip_prefix("std::").unwrap();

        // Strip glob suffix if present for checking
        let module_base = module_name.strip_suffix("::*").unwrap_or(module_name);

        // Windjammer stdlib `Map` → Rust HashMap (standalone crates have no windjammer_runtime::map).
        if module_base == "map" || module_base.starts_with("map::") {
            if module_name.ends_with("::Map") || module_name == "map::Map" {
                return Some("use std::collections::HashMap as Map;\n".to_string());
            }
            return Some(format!(
                "use std::collections::{};\n",
                module_name.replace("map::", "collections::")
            ));
        }

        let kind = crate::codegen::rust::stdlib_method_traits::classify_wj_std_import(module_base);
        match kind {
            crate::codegen::rust::stdlib_method_traits::WjStdImportKind::Skip => {
                return Some(String::new());
            }
            crate::codegen::rust::stdlib_method_traits::WjStdImportKind::RustStd => {
                if let Some(alias_name) = alias {
                    return Some(format!("use std::{} as {};\n", module_name, alias_name));
                }
                return Some(format!("use std::{};\n", module_name));
            }
            crate::codegen::rust::stdlib_method_traits::WjStdImportKind::Runtime { rust_stem } => {
                Some(
                    crate::codegen::rust::stdlib_method_traits::format_runtime_std_use(
                        module_name,
                        &rust_stem,
                        alias,
                    ),
                )
            }
        }
    }
}
