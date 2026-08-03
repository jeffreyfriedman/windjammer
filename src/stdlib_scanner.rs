//! Scans Rust source files to extract function signatures for the SignatureRegistry
//! This allows the compiler to know the ownership requirements of stdlib functions

use crate::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use crate::parser::Type;
use std::fs;
use std::path::Path;

/// Scan windjammer-runtime source files and populate the registry
pub fn populate_runtime_signatures(registry: &mut SignatureRegistry) -> Result<(), String> {
    let runtime_path = Path::new("crates/windjammer-runtime/src");

    if !runtime_path.exists() {
        // If runtime source isn't available (e.g., when installed via cargo),
        // fall back to hardcoded signatures
        return populate_fallback_signatures(registry);
    }

    // Scan all .rs files in runtime
    scan_directory(runtime_path, registry)?;

    Ok(())
}

fn scan_directory(path: &Path, registry: &mut SignatureRegistry) -> Result<(), String> {
    if !path.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(path).map_err(|e| format!("Failed to read directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
            scan_rust_file(&path, registry)?;
        }
    }

    Ok(())
}

fn scan_rust_file(path: &Path, registry: &mut SignatureRegistry) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    // Extract module name from file path (e.g., "game.rs" -> "game")
    let module_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // Skip lib.rs (just re-exports)
    if module_name == "lib" {
        return Ok(());
    }

    // Track `impl Type { ... }` so methods register as both `module::fn` and `Type::fn`
    // (call sites resolve `Connection::query`, not only `db::query`).
    let mut current_impl: Option<String> = None;
    let mut brace_depth: i32 = 0;
    let mut impl_depth: Option<i32> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(type_name) = parse_impl_type_name(trimmed) {
            current_impl = Some(type_name);
            // Depth after this line's braces is assigned below; mark entry depth.
            impl_depth = Some(brace_depth);
        }

        let opens = line.chars().filter(|&c| c == '{').count() as i32;
        let closes = line.chars().filter(|&c| c == '}').count() as i32;
        brace_depth += opens - closes;
        if let Some(start) = impl_depth {
            if brace_depth <= start {
                current_impl = None;
                impl_depth = None;
            }
        }

        if let Some(sig) = parse_function_signature(trimmed, module_name) {
            let method_name = sig
                .name
                .rsplit_once("::")
                .map(|(_, m)| m.to_string())
                .unwrap_or_else(|| sig.name.clone());
            registry.add_function(sig.name.clone(), sig.clone());
            if let Some(ref ty) = current_impl {
                let mut typed = sig;
                typed.name = format!("{ty}::{method_name}");
                registry.add_function(typed.name.clone(), typed);
            }
        }
    }

    Ok(())
}

/// `impl Connection` / `impl Connection {` / `impl<'a> Connection` → `Connection`.
fn parse_impl_type_name(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix("impl")?;
    let rest = rest.trim_start();
    // Skip lifetime/type generics on the impl itself: `impl<'a> Foo`
    let after_generics = if rest.starts_with('<') {
        let mut depth = 0;
        let mut end = None;
        for (i, c) in rest.char_indices() {
            match c {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        &rest[end? + 1..]
    } else {
        rest
    };
    let after_generics = after_generics.trim_start();
    // Skip `impl Trait for Type` — register under the concrete type after `for`.
    let type_part = if let Some(idx) = after_generics.find(" for ") {
        after_generics[idx + 5..].trim_start()
    } else {
        after_generics
    };
    let name = type_part
        .split(|c: char| c == '<' || c == '{' || c.is_whitespace())
        .next()
        .unwrap_or("")
        .trim();
    if name.is_empty() || !name.chars().next().is_some_and(|c| c.is_uppercase()) {
        return None;
    }
    Some(name.to_string())
}

fn parse_function_signature(line: &str, module: &str) -> Option<FunctionSignature> {
    let line = line.trim();

    // Must start with "pub fn"
    if !line.starts_with("pub fn ") {
        return None;
    }

    let after_fn = line.strip_prefix("pub fn ")?;
    let (func_name, params_str) = extract_rust_fn_name_and_params(after_fn)?;

    // Parse parameter ownership
    let param_ownership = parse_parameters(&params_str);
    let emitted_rust_ref_params = parse_emitted_rust_ref_flags(&params_str);
    let has_self_receiver = first_param_is_self_receiver(&params_str);

    // Build full name with module prefix
    let full_name = format!("{}::{}", module, func_name);

    Some(FunctionSignature {
        name: full_name,
        param_types: vec![], // TODO: Extract from Rust AST
        formal_param_types: vec![],
        param_ownership,
        return_type: None,                      // TODO: Extract from Rust AST
        return_ownership: OwnershipMode::Owned, // Default
        has_self_receiver,
        is_extern: false,
        emitted_rust_ref_params,
        field_extract_params: None,
        forwarding_borrow_params: None,
    })
}

fn first_param_is_self_receiver(params_str: &str) -> bool {
    let first = params_str.split(',').next().unwrap_or("").trim();
    matches!(
        first,
        "self" | "&self" | "&mut self" | "mut self"
    ) || first.starts_with("self:")
        || first.starts_with("&self")
        || first.starts_with("&mut self")
        || first.starts_with("mut self:")
}

/// Strip `name<'a>` / `name<T>` generics and extract the parameter list.
fn extract_rust_fn_name_and_params(after_fn: &str) -> Option<(String, String)> {
    let name_end = after_fn.find(|c| c == '<' || c == '(')?;
    let func_name = after_fn[..name_end].trim().to_string();
    let mut rest = &after_fn[name_end..];
    if rest.starts_with('<') {
        let mut depth = 0;
        let mut end = None;
        for (i, c) in rest.char_indices() {
            match c {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let close = end?;
        rest = &rest[close + 1..];
    }
    let open = rest.find('(')?;
    let params_start = open + 1;
    let mut depth = 0;
    let mut params_end = None;
    for (i, c) in rest[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    params_end = Some(open + i);
                    break;
                }
            }
            _ => {}
        }
    }
    let params_end = params_end?;
    Some((func_name, rest[params_start..params_end].to_string()))
}

fn parse_parameters(params_str: &str) -> Vec<OwnershipMode> {
    if params_str.trim().is_empty() {
        return Vec::new();
    }

    params_str
        .split(',')
        .map(|param| {
            let param = param.trim();

            // Check for &mut
            if param.contains("&mut ") {
                OwnershipMode::MutBorrowed
            }
            // impl AsRef<str> (db::connect, Connection::query, etc.)
            else if param.contains("AsRef<str>") {
                OwnershipMode::Borrowed
            }
            // Check for &
            else if param.contains('&') && !param.contains("&mut") {
                OwnershipMode::Borrowed
            }
            // Otherwise owned
            else {
                OwnershipMode::Owned
            }
        })
        .collect()
}

/// True when the scanned Rust formal lowers to shared borrow (`&str`, `&T`, `AsRef<str>`).
fn parse_emitted_rust_ref_flags(params_str: &str) -> Option<Vec<bool>> {
    if params_str.trim().is_empty() {
        return Some(vec![]);
    }
    Some(
        params_str
            .split(',')
            .map(|param| {
                let param = param.trim();
                param.contains("AsRef<str>")
                    || param.contains("&str")
                    || (param.contains('&') && !param.contains("&mut"))
            })
            .collect(),
    )
}

fn strings_split_signature(name: &str) -> FunctionSignature {
    FunctionSignature {
        name: name.to_string(),
        param_types: vec![Type::String, Type::String],
        formal_param_types: vec![Type::String, Type::String],
        param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
        return_type: Some(Type::Vec(Box::new(Type::String))),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
        emitted_rust_ref_params: Some(vec![true, true]),
        field_extract_params: None,
        forwarding_borrow_params: None,
    }
}

/// Fallback signatures when runtime source isn't available
fn populate_fallback_signatures(registry: &mut SignatureRegistry) -> Result<(), String> {
    use crate::parser::Type;
    use OwnershipMode::*;

    // Windjammer builtins - println macro/function
    registry.add_function(
        "println".to_string(),
        FunctionSignature {
            name: "println".to_string(),
            param_types: vec![Type::Reference(Box::new(Type::String))], // Takes &str
            formal_param_types: vec![],
            param_ownership: vec![Borrowed],
            return_type: None,
            return_ownership: Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        },
    );

    // std::game - ECS functions
    registry.add_function(
        "game::create_entity".to_string(),
        FunctionSignature {
            name: "game::create_entity".to_string(),
            param_types: vec![],
            formal_param_types: vec![],
            param_ownership: vec![MutBorrowed],
            return_type: None,
            return_ownership: Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        },
    );

    registry.add_function(
        "game::add_transform".to_string(),
        FunctionSignature {
            name: "game::add_transform".to_string(),
            param_types: vec![],
            formal_param_types: vec![],
            param_ownership: vec![MutBorrowed, Owned, Owned],
            return_type: None,
            return_ownership: Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        },
    );

    registry.add_function(
        "game::add_velocity".to_string(),
        FunctionSignature {
            name: "game::add_velocity".to_string(),
            param_types: vec![],
            formal_param_types: vec![],
            param_ownership: vec![MutBorrowed, Owned, Owned],
            return_type: None,
            return_ownership: Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        },
    );

    registry.add_function(
        "game::add_mesh".to_string(),
        FunctionSignature {
            name: "game::add_mesh".to_string(),
            param_types: vec![],
            formal_param_types: vec![],
            param_ownership: vec![MutBorrowed, Owned, Owned],
            return_type: None,
            return_ownership: Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        },
    );

    // windjammer_runtime::strings::split - used via "use ...::split"
    // Return type critical for Vec<String> indexing: let lines = split(...); let line = lines[i]
    registry.add_function("split".to_string(), strings_split_signature("split"));
    registry.add_function(
        "strings::split".to_string(),
        strings_split_signature("strings::split"),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_function_signature() {
        let line = "pub fn create_entity(world: &mut World) -> EntityId {";
        let sig = parse_function_signature(line, "game").unwrap();

        assert_eq!(sig.name, "game::create_entity");
        assert_eq!(sig.param_ownership.len(), 1);
        assert_eq!(sig.param_ownership[0], OwnershipMode::MutBorrowed);
    }

    #[test]
    fn test_parse_multiple_params() {
        let line =
            "pub fn add_component(world: &mut World, entity: EntityId, component: Transform) {";
        let sig = parse_function_signature(line, "game").unwrap();

        assert_eq!(sig.param_ownership.len(), 3);
        assert_eq!(sig.param_ownership[0], OwnershipMode::MutBorrowed);
        assert_eq!(sig.param_ownership[1], OwnershipMode::Owned);
        assert_eq!(sig.param_ownership[2], OwnershipMode::Owned);
    }

    #[test]
    fn test_parse_borrowed_param() {
        let line = "pub fn query(world: &World) -> Vec<EntityId> {";
        let sig = parse_function_signature(line, "game").unwrap();

        assert_eq!(sig.param_ownership.len(), 1);
        assert_eq!(sig.param_ownership[0], OwnershipMode::Borrowed);
        assert!(!sig.has_self_receiver);
    }

    #[test]
    fn connection_query_marks_self_receiver_so_vec_params_stay_owned() {
        let line =
            "pub fn query(&self, sql: impl AsRef<str>, params: Vec<String>) -> Result<Vec<Row>, String> {";
        let sig = parse_function_signature(line, "db").unwrap();

        assert!(sig.has_self_receiver);
        assert_eq!(sig.param_ownership.len(), 3);
        assert_eq!(sig.param_ownership[0], OwnershipMode::Borrowed); // &self
        assert_eq!(sig.param_ownership[1], OwnershipMode::Borrowed); // AsRef<str>
        assert_eq!(sig.param_ownership[2], OwnershipMode::Owned); // Vec<String>
        // User arg 1 (params) must map to Owned — not sql's Borrowed (off-by-one without self).
        assert_eq!(
            sig.param_ownership[sig.arg_param_index(1)],
            OwnershipMode::Owned
        );
    }

    #[test]
    fn parse_impl_type_name_extracts_connection() {
        assert_eq!(
            parse_impl_type_name("impl Connection {"),
            Some("Connection".into())
        );
        assert_eq!(
            parse_impl_type_name("impl<'a> Connection {"),
            Some("Connection".into())
        );
    }

    #[test]
    fn strings_split_scan_sets_delimiter_ref_flag() {
        let line = "pub fn split<S: AsRef<str>>(s: S, delimiter: &str) -> Vec<String> {";
        let sig = parse_function_signature(line, "strings").unwrap();
        assert_eq!(
            sig.emitted_rust_ref_params,
            Some(vec![true, true]),
            "AsRef<str> and &str formals must emit shared refs at call sites"
        );
    }
}
