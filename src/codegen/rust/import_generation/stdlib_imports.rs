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
                let rust_import = format!("windjammer_runtime::{rust_stem}");
                if let Some(alias_name) = alias {
                    return Some(format!("use {} as {};\n", rust_import, alias_name));
                }
                if rust_stem.ends_with("_mod") || rust_stem.ends_with("_runtime") {
                    let original_name = rust_stem
                        .strip_suffix("_mod")
                        .or_else(|| rust_stem.strip_suffix("_runtime"))
                        .unwrap_or(&rust_stem);
                    let mut result = format!("use {} as {};\n", rust_import, original_name);
                    let registry = crate::analyzer::SignatureRegistry::stdlib();
                    for ty in registry.runtime_exported_types_for_module(original_name) {
                        result.push_str(&format!("use {rust_import}::{ty};\n"));
                    }
                    return Some(result);
                }
                let rest = module_name
                    .strip_prefix(module_base.split("::").next().unwrap_or(module_base))
                    .unwrap_or("");
                return Some(format!("use {rust_import}{rest};\n"));
            }
        }
    }
}
