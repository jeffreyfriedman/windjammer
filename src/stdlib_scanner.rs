//! Scans Rust source files to extract function signatures for the SignatureRegistry
//! This allows the compiler to know the ownership requirements of stdlib functions

use crate::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
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

    // Simple regex-based parsing of public functions
    // Look for patterns like: pub fn function_name(param: &mut Type, ...)
    for line in content.lines() {
        if let Some(sig) = parse_function_signature(line, module_name) {
            registry.add_function(sig.name.clone(), sig);
        }
    }

    Ok(())
}

fn parse_function_signature(line: &str, module: &str) -> Option<FunctionSignature> {
    let line = line.trim();

    // Must start with "pub fn"
    if !line.starts_with("pub fn ") {
        return None;
    }

    // Extract function name
    let after_fn = line.strip_prefix("pub fn ")?;
    let name_end = after_fn.find('(')?;
    let func_name = &after_fn[..name_end];

    // Extract parameters (between parentheses)
    let params_start = after_fn.find('(')?;
    let params_end = after_fn.find(')')?;
    let params_str = &after_fn[params_start + 1..params_end];

    // Parse parameter ownership
    let param_ownership = parse_parameters(params_str);

    // Build full name with module prefix
    let full_name = format!("{}::{}", module, func_name);

    Some(FunctionSignature {
        name: full_name,
        param_types: vec![], // TODO: Extract from Rust AST
        param_ownership,
        return_type: None,                      // TODO: Extract from Rust AST
        return_ownership: OwnershipMode::Owned, // Default
        has_self_receiver: false,               // Stdlib functions don't have self
        is_extern: false,                       // Stdlib functions are not extern
    })
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
            param_ownership: vec![Borrowed],
            return_type: None,
            return_ownership: Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    // std::game - ECS functions
    registry.add_function(
        "game::create_entity".to_string(),
        FunctionSignature {
            name: "game::create_entity".to_string(),
            param_types: vec![],
            param_ownership: vec![MutBorrowed],
            return_type: None,
            return_ownership: Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    registry.add_function(
        "game::add_transform".to_string(),
        FunctionSignature {
            name: "game::add_transform".to_string(),
            param_types: vec![],
            param_ownership: vec![MutBorrowed, Owned, Owned],
            return_type: None,
            return_ownership: Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    registry.add_function(
        "game::add_velocity".to_string(),
        FunctionSignature {
            name: "game::add_velocity".to_string(),
            param_types: vec![],
            param_ownership: vec![MutBorrowed, Owned, Owned],
            return_type: None,
            return_ownership: Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    registry.add_function(
        "game::add_mesh".to_string(),
        FunctionSignature {
            name: "game::add_mesh".to_string(),
            param_types: vec![],
            param_ownership: vec![MutBorrowed, Owned, Owned],
            return_type: None,
            return_ownership: Owned,
            has_self_receiver: false,
            is_extern: false,
        },
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
    }
}
