//! The library build's `pub use` re-export table, and the re-export keys of the library field table: a struct
//! literal written through a re-export (`ffi::GpuVertex`) finds its field types (IR solver, numeric inference).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The `pub use` re-exports of every module (declaring module -> exported name -> struct key), with every
/// re-exported struct also keyed under its re-export paths in the library field table
/// (`register_reexport_struct_aliases`).
pub(super) fn collect_module_reexports(
    sources: &[(PathBuf, String)],
    parsed_programs: &[crate::parser::Program<'static>],
    src_base: &Path,
    global_struct_fields: &mut std::sync::Arc<HashMap<String, HashMap<String, crate::parser::ast::types::Type>>>,
    struct_defining_module_paths: &HashMap<String, Vec<Vec<String>>>,
) -> HashMap<String, HashMap<String, String>> {
    // Collect `pub use` re-exports from every file first so `use super::*` / `use crate::...::*`
    // can resolve struct field types (glob has no explicit type path).
    let mut module_re_exports: HashMap<String, HashMap<String, String>> = HashMap::new();
    for (i, (file, _source)) in sources.iter().enumerate() {
        let program = &parsed_programs[i];
        let file_module = crate::analyzer::type_collector::wj_file_to_module_path(src_base, file)
            .unwrap_or_default();
        crate::type_inference::struct_field_registry::merge_module_reexports_from_items(
            &program.items,
            &file_module,
            global_struct_fields,
            struct_defining_module_paths,
            &mut module_re_exports,
        );
        if crate::type_inference::struct_field_registry::debug_struct_import_trace()
            && file.to_string_lossy().contains("dialogue")
        {
            eprintln!(
                "=== WJ_DEBUG: file={} file_module_path={:?}",
                file.display(),
                file_module
            );
        }
    }

    // A struct literal written through a re-export (`ffi::GpuVertex`) finds its fields (IR solver, numeric inference).
    crate::type_inference::struct_field_registry::register_reexport_struct_aliases(
        std::sync::Arc::make_mut(global_struct_fields),
        &module_re_exports,
        struct_defining_module_paths,
    );

    if crate::type_inference::struct_field_registry::debug_struct_import_trace() {
        eprintln!("=== GLOBAL MODULE_RE_EXPORTS (post pre-pass) ===");
        let mut mods: Vec<_> = module_re_exports.keys().cloned().collect();
        mods.sort();
        for m in &mods {
            if !m.contains("dialogue") && !m.is_empty() {
                continue;
            }
            let exports = &module_re_exports[m];
            eprintln!("  module {:?}: {} exports", m, exports.len());
            for (name, key) in exports {
                if name.contains("Dialogue") {
                    eprintln!("    {} → {}", name, key);
                }
            }
        }
    }
    module_re_exports
}
