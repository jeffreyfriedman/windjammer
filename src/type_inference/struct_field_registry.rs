//! Module-qualified keys for struct field type registries (multipass library builds).

use crate::parser::ast::core::Item;
use crate::parser::ast::types::Type;
use std::collections::HashMap;

/// Set `WJ_DEBUG_GLOB_IMPORT=1` to trace `pub use`, glob imports, and use-path resolution.
pub(crate) fn debug_struct_import_trace() -> bool {
    matches!(
        std::env::var("WJ_DEBUG_GLOB_IMPORT").ok().as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

fn trace_import(msg: impl AsRef<str>) {
    if debug_struct_import_trace() {
        eprintln!("{}", msg.as_ref());
    }
}

/// Build registry key: `a::b::StructName` from file/nested-mod prefix and struct name.
pub fn qualify_struct_key(module_prefix: &[String], struct_name: &str) -> String {
    if module_prefix.is_empty() {
        struct_name.to_string()
    } else {
        format!("{}::{}", module_prefix.join("::"), struct_name)
    }
}

/// Resolve `use` path (last segment = type name) to a key present in `struct_field_types`.
pub fn resolve_use_path_to_qualified_key(
    path: &[String],
    current_module: &[String],
    struct_field_types: &HashMap<String, HashMap<String, Type>>,
    defining_paths: &HashMap<String, Vec<Vec<String>>>,
) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    if path.len() == 1 && path[0].contains("::{") {
        return None;
    }
    let type_name = path.last()?.as_str();
    let module_prefix = if path.len() == 1 {
        &[][..]
    } else {
        &path[..path.len() - 1]
    };

    let try_key = |m: &[String]| -> Option<String> {
        let k = qualify_struct_key(m, type_name);
        if struct_field_types.contains_key(&k) {
            Some(k)
        } else {
            None
        }
    };

    // `pub use system::Foo` inside `dialogue/mod.wj` is resolved relative to `dialogue`, not
    // as crate-root `system::Foo`. Try current-module-relative first, then crate-root segments.
    let resolved_candidates: Vec<Vec<String>> = match module_prefix.first().map(|s| s.as_str()) {
        None | Some("crate") | Some("super") | Some("self") => {
            vec![resolve_module_prefix_from_use_path(
                module_prefix,
                current_module,
            )]
        }
        Some(_) => {
            let rel = append_module_prefix_relative_to_current(current_module, module_prefix);
            let abs = module_prefix.to_vec();
            if rel == abs {
                vec![rel]
            } else {
                vec![rel, abs]
            }
        }
    };

    for resolved_module in &resolved_candidates {
        if debug_struct_import_trace() {
            trace_import(format!(
                "  resolve_use_path: type={} current={:?} module_prefix={:?} try_module={:?}",
                type_name, current_module, module_prefix, resolved_module
            ));
        }
        if let Some(k) = try_key(resolved_module) {
            if debug_struct_import_trace() {
                trace_import(format!("    -> hit registry key {}", k));
            }
            return Some(k);
        }
    }

    if let Some(candidates) = defining_paths.get(type_name) {
        for cand in candidates {
            for rc in &resolved_candidates {
                if cand.len() >= rc.len() && cand[..rc.len()] == rc[..] {
                    if let Some(k) = try_key(cand) {
                        return Some(k);
                    }
                    break;
                }
            }
        }
        if candidates.len() == 1 {
            return try_key(&candidates[0]);
        }
    }

    None
}

/// `use foo::{A, B};` → `path` is a single string like `foo::{A, B}`.
pub fn register_braced_use_imports(
    combined: &str,
    current_module: &[String],
    struct_field_types: &HashMap<String, HashMap<String, Type>>,
    defining_paths: &HashMap<String, Vec<Vec<String>>>,
    out: &mut HashMap<String, String>,
) {
    let Some((mod_part, rest)) = combined.split_once("::{") else {
        return;
    };
    let Some(inner) = rest.strip_suffix('}') else {
        return;
    };
    let module_segments: Vec<String> = mod_part
        .split("::")
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    for raw in inner.split(',') {
        let typ = raw.trim();
        if typ.is_empty() || typ == "*" {
            continue;
        }
        let mut p = module_segments.clone();
        p.push(typ.to_string());
        if let Some(key) = resolve_use_path_to_qualified_key(
            &p,
            current_module,
            struct_field_types,
            defining_paths,
        ) {
            out.insert(typ.to_string(), key);
        }
    }
}

/// `pub use foo::{A, B};` from a module — record each exported name → qualified struct key.
pub fn register_braced_pub_use_reexports(
    combined: &str,
    declaring_module: &[String],
    struct_field_types: &HashMap<String, HashMap<String, Type>>,
    defining_paths: &HashMap<String, Vec<Vec<String>>>,
    module_re_exports: &mut HashMap<String, HashMap<String, String>>,
) {
    let Some((mod_part, rest)) = combined.split_once("::{") else {
        return;
    };
    let Some(inner) = rest.strip_suffix('}') else {
        return;
    };
    let module_segments: Vec<String> = mod_part
        .split("::")
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    let module_key = declaring_module.join("::");
    let exports = module_re_exports.entry(module_key).or_default();
    for raw in inner.split(',') {
        let typ = raw.trim();
        if typ.is_empty() || typ == "*" {
            continue;
        }
        let mut p = module_segments.clone();
        p.push(typ.to_string());
        if let Some(key) = resolve_use_path_to_qualified_key(
            &p,
            declaring_module,
            struct_field_types,
            defining_paths,
        ) {
            exports.insert(typ.to_string(), key);
        }
    }
}

/// Walk items (and inline `mod` blocks) and merge `pub use` re-exports into `out`.
pub fn merge_module_reexports_from_items<'ast>(
    items: &[Item<'ast>],
    declaring_module: &[String],
    struct_field_types: &HashMap<String, HashMap<String, Type>>,
    defining_paths: &HashMap<String, Vec<Vec<String>>>,
    out: &mut HashMap<String, HashMap<String, String>>,
) {
    if debug_struct_import_trace() {
        trace_import(format!(
            "=== merge_module_reexports_from_items: declaring_module={:?}",
            declaring_module
        ));
    }
    for item in items {
        match item {
            Item::Use {
                path,
                alias,
                is_pub: true,
                ..
            } => {
                if path.len() == 1 && path[0].contains("::{") {
                    if debug_struct_import_trace() {
                        trace_import(format!("  pub use (braced): {}", path[0]));
                    }
                    register_braced_pub_use_reexports(
                        &path[0],
                        declaring_module,
                        struct_field_types,
                        defining_paths,
                        out,
                    );
                    continue;
                }
                if path.last().map(|s| s.as_str()) == Some("*") {
                    continue;
                }
                if path.len() < 2 {
                    continue;
                }
                if debug_struct_import_trace() {
                    trace_import(format!("  pub use: path={:?}", path));
                }
                if let Some(key) = resolve_use_path_to_qualified_key(
                    path,
                    declaring_module,
                    struct_field_types,
                    defining_paths,
                ) {
                    let exported_name = alias
                        .clone()
                        .unwrap_or_else(|| path.last().cloned().unwrap_or_default());
                    let module_key = declaring_module.join("::");
                    if debug_struct_import_trace() {
                        trace_import(format!(
                            "    -> re-export {} → {} (module_key={:?})",
                            exported_name, key, module_key
                        ));
                    }
                    out.entry(module_key)
                        .or_default()
                        .insert(exported_name, key);
                } else if debug_struct_import_trace() {
                    trace_import(format!(
                        "    -> FAILED to resolve pub use path={:?} from {:?}",
                        path, declaring_module
                    ));
                }
            }
            Item::Mod {
                name,
                items: nested,
                ..
            } => {
                let mut next = declaring_module.to_vec();
                next.push(name.clone());
                merge_module_reexports_from_items(
                    nested,
                    &next,
                    struct_field_types,
                    defining_paths,
                    out,
                );
            }
            _ => {}
        }
    }
}

/// `use path::to::module::*` — register imported struct keys for that module's public namespace.
pub fn expand_glob_import(
    path: &[String],
    current_module: &[String],
    struct_field_types: &HashMap<String, HashMap<String, Type>>,
    _defining_paths: &HashMap<String, Vec<Vec<String>>>,
    module_re_exports: &HashMap<String, HashMap<String, String>>,
    out: &mut HashMap<String, String>,
) {
    if debug_struct_import_trace() {
        trace_import(format!(
            "=== expand_glob_import path={:?} current_module={:?}",
            path, current_module
        ));
    }
    if path.is_empty() || path.last().map(|s| s.as_str()) != Some("*") {
        if debug_struct_import_trace() {
            trace_import("  skip: not a glob import");
        }
        return;
    }
    let module_path = &path[..path.len() - 1];
    let resolved_module = resolve_module_prefix_from_use_path(module_path, current_module);
    let module_key = resolved_module.join("::");

    if debug_struct_import_trace() {
        trace_import(format!(
            "  resolved parent module_key={:?} (segments={:?})",
            module_key, resolved_module
        ));
    }

    let prefix_with_sep = if module_key.is_empty() {
        String::new()
    } else {
        format!("{}::", module_key)
    };

    let mut n_direct = 0usize;
    // 1) Structs defined directly in this module (exactly one path segment after the module prefix)
    for (key, _) in struct_field_types {
        if module_key.is_empty() {
            if !key.contains("::") {
                out.insert(key.clone(), key.clone());
            }
        } else if let Some(rest) = key.strip_prefix(&prefix_with_sep) {
            if !rest.contains("::") {
                out.insert(rest.to_string(), key.clone());
                n_direct += 1;
            }
        }
    }

    // 2) `pub use` re-exports from that module (overwrite: re-export wins over same bare name)
    let mut n_reexport = 0usize;
    if let Some(exports) = module_re_exports.get(&module_key) {
        if debug_struct_import_trace() {
            trace_import(format!(
                "  re-exports from {:?}: {} names",
                module_key,
                exports.len()
            ));
        }
        for (name, qkey) in exports {
            if debug_struct_import_trace() && name.contains("Dialogue") {
                trace_import(format!("    {} → {}", name, qkey));
            }
            out.insert(name.clone(), qkey.clone());
            n_reexport += 1;
        }
    } else if debug_struct_import_trace() {
        trace_import(format!(
            "  no re-exports map entry for module_key={:?}",
            module_key
        ));
    }

    if debug_struct_import_trace() {
        trace_import(format!(
            "  glob done: {} direct-under-module types, {} re-export bindings merged into import map",
            n_direct, n_reexport
        ));
    }
}

fn append_module_prefix_relative_to_current(
    current_module: &[String],
    module_path: &[String],
) -> Vec<String> {
    let mut out = current_module.to_vec();
    for seg in module_path {
        push_module_segment(&mut out, seg);
    }
    out
}

fn resolve_module_prefix_from_use_path(
    module_path: &[String],
    current_module: &[String],
) -> Vec<String> {
    if module_path.is_empty() {
        return current_module.to_vec();
    }
    match module_path[0].as_str() {
        "crate" => {
            let mut out = Vec::new();
            let mut i = 1;
            while i < module_path.len() {
                push_module_segment(&mut out, &module_path[i]);
                i += 1;
            }
            out
        }
        "super" => {
            let mut out = current_module.to_vec();
            if !out.is_empty() {
                out.pop();
            }
            let mut i = 1;
            while i < module_path.len() {
                push_module_segment(&mut out, &module_path[i]);
                i += 1;
            }
            out
        }
        "self" => {
            let mut out = current_module.to_vec();
            let mut i = 1;
            while i < module_path.len() {
                push_module_segment(&mut out, &module_path[i]);
                i += 1;
            }
            out
        }
        _ => module_path.to_vec(),
    }
}

fn push_module_segment(out: &mut Vec<String>, seg: &str) {
    match seg {
        "super" => {
            if !out.is_empty() {
                out.pop();
            }
        }
        "self" => {}
        s => out.push(s.to_string()),
    }
}

/// Lookup field map for a struct type name (possibly qualified, possibly imported).
pub fn lookup_struct_field_map<'a>(
    struct_field_types: &'a HashMap<String, HashMap<String, Type>>,
    type_name: &str,
    imported_type_keys: &HashMap<String, String>,
    defining_paths: &HashMap<String, Vec<Vec<String>>>,
) -> Option<&'a HashMap<String, Type>> {
    let base_name = if let Some(idx) = type_name.find('<') {
        &type_name[..idx]
    } else {
        type_name
    };

    if base_name.contains("::") {
        if let Some(m) = struct_field_types.get(base_name) {
            return Some(m);
        }
    }

    if let Some(k) = imported_type_keys.get(base_name) {
        if let Some(m) = struct_field_types.get(k) {
            return Some(m);
        }
    }

    if let Some(paths) = defining_paths.get(base_name) {
        if paths.len() == 1 {
            let k = qualify_struct_key(&paths[0], base_name);
            if let Some(m) = struct_field_types.get(&k) {
                return Some(m);
            }
        }
    }

    if let Some(m) = struct_field_types.get(base_name) {
        return Some(m);
    }

    // Single-file nested `mod foo { struct Bar ... }`: registry key is `foo::Bar` but literals use `Bar`.
    // Only use this when unambiguous (exactly one `*::Bar` key).
    if !base_name.contains("::") {
        let suffix = format!("::{}", base_name);
        let mut hit: Option<&'a HashMap<String, Type>> = None;
        for (k, v) in struct_field_types.iter() {
            if k.ends_with(&suffix) {
                if hit.is_some() {
                    return None;
                }
                hit = Some(v);
            }
        }
        if let Some(m) = hit {
            return Some(m);
        }
    }

    None
}
