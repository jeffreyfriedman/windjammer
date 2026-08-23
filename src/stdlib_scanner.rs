//! Scans Rust source files to extract function signatures for the SignatureRegistry
//! This allows the compiler to know the ownership requirements of stdlib functions

use crate::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use crate::parser::Type;
use std::fs;
use std::path::Path;

/// Resolve `windjammer-runtime/src` for signature scanning.
///
/// Must not depend on process CWD: `wj build` / `wj test` run from user packages
/// (ecosystem seeds, apps). CWD-relative `crates/windjammer-runtime/src` only works
/// inside the compiler repo and silently falls back to incomplete signatures otherwise
/// (e.g. `strings::join` → spurious `parts.clone()` into `&[String]`).
pub(crate) fn resolve_runtime_src_for_scan() -> Option<std::path::PathBuf> {
    let from_finder = crate::cargo_toml::find_windjammer_runtime_path().join("src");
    if from_finder.is_dir() {
        return Some(from_finder);
    }
    // Dev convenience when CWD is the windjammer crate root.
    let cwd_relative = Path::new("crates/windjammer-runtime/src");
    if cwd_relative.is_dir() {
        return Some(cwd_relative.to_path_buf());
    }
    None
}

/// Scan windjammer-runtime source files and populate the registry
pub fn populate_runtime_signatures(registry: &mut SignatureRegistry) -> Result<(), String> {
    let Some(runtime_path) = resolve_runtime_src_for_scan() else {
        // If runtime source isn't available (e.g., incomplete install),
        // fall back to hardcoded signatures
        return populate_fallback_signatures(registry);
    };

    // Scan all .rs files in runtime
    scan_directory(&runtime_path, registry)?;
    register_wj_std_module_names(registry);
    register_runtime_modules_from_signature_keys(registry);

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

    registry.register_runtime_file_stem(module_name);

    // Track `impl Type { ... }` so methods register as `Type::fn` (free functions
    // keep exclusive claim on `module::fn` — see `register_scanned_runtime_signature`).
    let mut current_impl: Option<String> = None;
    let mut brace_depth: i32 = 0;
    let mut impl_depth: Option<i32> = None;
    let mut pending_sanitizer = false;
    let mut current_struct: Option<String> = None;
    let mut struct_fields: Vec<(String, String)> = Vec::new();
    let mut struct_body_depth: Option<i32> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if is_wj_taint_sanitizer_comment(trimmed) {
            pending_sanitizer = true;
        } else if !trimmed.is_empty()
            && !trimmed.starts_with("//")
            && !trimmed.starts_with("///")
            && !trimmed.starts_with("#[")
            && parse_function_signature(trimmed, module_name).is_none()
        {
            pending_sanitizer = false;
        }

        if brace_depth == 0 {
            if let Some(type_name) = parse_exported_type_name(trimmed) {
                registry.register_runtime_exported_type(module_name, &type_name);
            }
            if current_struct.is_none() {
                if let Some(struct_name) = parse_named_struct_start(trimmed) {
                    current_struct = Some(struct_name);
                    struct_fields.clear();
                    struct_body_depth = None;
                }
            }
        }
        if let Some(type_name) = parse_impl_type_name(trimmed) {
            current_impl = Some(type_name);
            // Depth after this line's braces is assigned below; mark entry depth.
            impl_depth = Some(brace_depth);
        }

        if current_struct.is_some() {
            if let Some(field) = parse_pub_struct_field(trimmed) {
                struct_fields.push(field);
            }
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
        if current_struct.is_some() && struct_body_depth.is_none() && brace_depth > 0 {
            struct_body_depth = Some(brace_depth);
        }
        if let Some(body_depth) = struct_body_depth {
            if brace_depth < body_depth {
                if let Some(name) = current_struct.take() {
                    registry
                        .register_runtime_type_fields(&name, std::mem::take(&mut struct_fields));
                }
                struct_body_depth = None;
            }
        }

        if let Some(sig) = parse_function_signature(trimmed, module_name) {
            let mark_sanitizer = pending_sanitizer;
            pending_sanitizer = false;
            register_scanned_runtime_signature(
                registry,
                module_name,
                sig,
                current_impl.as_deref(),
                mark_sanitizer,
            );
        }
    }

    Ok(())
}

/// Register a scanned runtime fn under `module::name`, optional `*_mod` → short alias,
/// and `Type::name` when inside an `impl`.
///
/// Self methods register only as `Type::method`. Dual-registering them under
/// `module::fn` clobbers free functions with the same name (`http::post` vs
/// `Router::post`) — call sites then borrow only the first user arg.
fn register_scanned_runtime_signature(
    registry: &mut SignatureRegistry,
    module_name: &str,
    sig: FunctionSignature,
    current_impl: Option<&str>,
    mark_sanitizer: bool,
) {
    let method_name = sig
        .name
        .rsplit_once("::")
        .map(|(_, m)| m.to_string())
        .unwrap_or_else(|| sig.name.clone());
    // Associated fns without `self` still share the module path (`Type::new`-style).
    let register_module_path = current_impl.is_none() || !sig.has_self_receiver;
    if register_module_path {
        if mark_sanitizer {
            registry.register_taint_sanitizer(&sig.name);
        }
        registry.add_function(sig.name.clone(), sig.clone());
        // Mirror import aliases: `csv_mod` / `regex_mod` → WJ short names `csv` / `regex`.
        if let Some(short) = module_name.strip_suffix("_mod") {
            let mut aliased = sig.clone();
            aliased.name = format!("{short}::{method_name}");
            if mark_sanitizer {
                registry.register_taint_sanitizer(&aliased.name);
            }
            registry.add_function(aliased.name.clone(), aliased);
            registry.register_runtime_std_module(short);
        }
    }
    registry.register_runtime_file_stem(module_name);
    if let Some(ty) = current_impl {
        let mut typed = sig;
        typed.name = format!("{ty}::{method_name}");
        if mark_sanitizer {
            registry.register_taint_sanitizer(&typed.name);
        }
        registry.add_function(typed.name.clone(), typed);
        registry.register_runtime_type_module(ty, module_name);
    }
}

fn is_wj_taint_sanitizer_comment(trimmed: &str) -> bool {
    let body = trimmed
        .strip_prefix("///")
        .or_else(|| trimmed.strip_prefix("//"))
        .unwrap_or("")
        .trim();
    body.eq_ignore_ascii_case("wj-taint: sanitizer") || body.contains("wj-taint: sanitizer")
}

fn parse_named_struct_start(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix("pub struct ")?;
    if rest.contains('(') {
        return None;
    }
    let name = rest
        .split(|c: char| c == '<' || c == '{' || c.is_whitespace())
        .next()
        .unwrap_or("")
        .trim();
    if name.is_empty() || !name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
        return None;
    }
    Some(name.to_string())
}

fn parse_pub_struct_field(trimmed: &str) -> Option<(String, String)> {
    let rest = trimmed.strip_prefix("pub ")?;
    if rest.starts_with("fn ")
        || rest.starts_with("struct ")
        || rest.starts_with("enum ")
        || rest.starts_with("use ")
        || rest.starts_with("const ")
        || rest.starts_with("type ")
        || rest.starts_with("impl ")
        || rest.starts_with("async ")
    {
        return None;
    }
    let (name, ty) = rest.split_once(':')?;
    let name = name.trim();
    if name.is_empty()
        || !name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
    {
        return None;
    }
    let ty = ty.trim().trim_end_matches(',').trim();
    if ty.is_empty() {
        return None;
    }
    Some((name.to_string(), ty.to_string()))
}

/// Lowercase `module::fn` keys (and `_mod` / `_runtime` aliases) → runtime module set.
fn register_runtime_modules_from_signature_keys(registry: &mut SignatureRegistry) {
    let keys: Vec<String> = registry.signatures.keys().cloned().collect();
    for name in keys {
        if let Some((module, _)) = name.split_once("::") {
            if module
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_lowercase())
            {
                registry.register_runtime_std_module(module);
            }
        }
    }
}

/// WJ `std/*.wj` and `std/*/mod.wj` stems (`async`, `process`, …) so aliases that
/// have no matching runtime file stem still classify as runtime std modules.
fn register_wj_std_module_names(registry: &mut SignatureRegistry) {
    let candidates = [
        Path::new("std").to_path_buf(),
        Path::new(env!("CARGO_MANIFEST_DIR")).join("std"),
    ];
    for dir in &candidates {
        if !dir.is_dir() {
            continue;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.join("mod.wj").is_file() {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        if is_wj_std_module_stem(name) {
                            registry.register_runtime_std_module(name);
                        }
                    }
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("wj") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if is_wj_std_module_stem(stem) {
                        registry.register_runtime_std_module(stem);
                    }
                }
            }
        }
        return;
    }
}

fn is_wj_std_module_stem(stem: &str) -> bool {
    !stem.ends_with("_test") && stem != "test_simple" && stem != "basic_test"
}

/// `pub struct Foo` / `pub enum Bar` / `pub use path::Baz` / `pub use path::Qux as Alias`.
fn parse_exported_type_name(trimmed: &str) -> Option<String> {
    let ident_from = |rest: &str| {
        let name = rest
            .split(|c: char| c == '<' || c == '{' || c == '(' || c == ';' || c.is_whitespace())
            .next()
            .unwrap_or("")
            .trim();
        if name.is_empty() || !name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
            None
        } else {
            Some(name.to_string())
        }
    };
    if let Some(rest) = trimmed.strip_prefix("pub struct ") {
        return ident_from(rest);
    }
    if let Some(rest) = trimmed.strip_prefix("pub enum ") {
        return ident_from(rest);
    }
    if let Some(rest) = trimmed.strip_prefix("pub use ") {
        let rest = rest.trim_end_matches(';').trim();
        if rest.contains('{') || rest.ends_with('*') {
            return None;
        }
        let name = if let Some((_, alias)) = rest.split_once(" as ") {
            alias.trim()
        } else {
            rest.rsplit("::").next().unwrap_or(rest).trim()
        };
        return ident_from(name);
    }
    None
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
    let (func_name, generics_str, params_str, after_params) =
        extract_rust_fn_name_generics_and_params(after_fn)?;

    // Type params bounded `S: AsRef<_>` (runtime fs/strings/db helpers) borrow at call sites.
    let asref_borrow_type_params = asref_borrow_generic_type_params(&generics_str);
    let asref_path_type_params = asref_path_type_params(&generics_str);

    // Parse parameter ownership
    let param_ownership = parse_parameters(&params_str, &asref_borrow_type_params);
    let emitted_rust_ref_params =
        parse_emitted_rust_ref_flags(&params_str, &asref_borrow_type_params);
    let param_types = parse_param_types(
        &params_str,
        &asref_borrow_type_params,
        &asref_path_type_params,
    );
    let has_self_receiver = first_param_is_self_receiver(&params_str);

    // Build full name with module prefix
    let full_name = format!("{}::{}", module, func_name);

    Some(FunctionSignature {
        name: full_name,
        param_types,
        formal_param_types: vec![],
        param_ownership,
        return_type: parse_return_type_after_params(&after_params),
        return_ownership: OwnershipMode::Owned, // Default
        has_self_receiver,
        is_extern: false,
        emitted_rust_ref_params,
        string_ref_string_formal_params: None,
        field_extract_params: None,
        forwarding_borrow_params: None,
    })
}

fn first_param_is_self_receiver(params_str: &str) -> bool {
    let first = params_str.split(',').next().unwrap_or("").trim();
    matches!(first, "self" | "&self" | "&mut self" | "mut self")
        || first.starts_with("self:")
        || first.starts_with("&self")
        || first.starts_with("&mut self")
        || first.starts_with("mut self:")
}

/// Strip `name<'a>` / `name<T>` generics and extract `(name, generics, params, after_params)`.
fn extract_rust_fn_name_generics_and_params(
    after_fn: &str,
) -> Option<(String, String, String, String)> {
    let name_end = after_fn.find(|c| c == '<' || c == '(')?;
    let func_name = after_fn[..name_end].trim().to_string();
    let mut rest = &after_fn[name_end..];
    let mut generics_str = String::new();
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
        // Keep inner generics (`S: AsRef<str>`) without the outer `<…>`.
        generics_str = rest[1..close].to_string();
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
    Some((
        func_name,
        generics_str,
        rest[params_start..params_end].to_string(),
        rest[params_end + 1..].to_string(),
    ))
}

fn asref_target_type_name(clause_or_param: &str) -> Option<&str> {
    let start = clause_or_param.find("AsRef<")?;
    let rest = &clause_or_param[start + "AsRef<".len()..];
    let mut depth = 1i32;
    for (i, c) in rest.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return Some(rest[..i].trim());
                }
            }
            _ => {}
        }
    }
    None
}

fn is_asref_path_target(target: &str) -> bool {
    target == "Path" || target.ends_with("::Path")
}

/// Type parameters with an `AsRef<_>` bound that accepts borrowed inputs at call sites.
///
/// Runtime helpers use `fn write<P: AsRef<Path>>(path: P)` / `fn len<S: AsRef<str>>(s: S)` —
/// the borrow contract is on the generic bound, not the formal (`path: P` alone looks owned).
fn asref_borrow_generic_type_params(generics_str: &str) -> Vec<String> {
    asref_borrow_bounds(generics_str)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

/// `(type_param, AsRef target)` pairs from fn generics (`P` → `Path`, `S` → `str`).
fn asref_borrow_bounds(generics_str: &str) -> Vec<(String, String)> {
    if generics_str.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for clause in split_top_level_commas(generics_str) {
        let clause = clause.trim();
        let Some(target) = asref_target_type_name(clause) else {
            continue;
        };
        let name = clause.split([':', '+']).next().unwrap_or("").trim();
        if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            out.push((name.to_string(), target.to_string()));
        }
    }
    out
}

fn asref_path_type_params(generics_str: &str) -> Vec<String> {
    asref_borrow_bounds(generics_str)
        .into_iter()
        .filter_map(|(name, target)| is_asref_path_target(&target).then_some(name))
        .collect()
}

fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth: i32 = 0;
    for (i, c) in s.char_indices() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
}

fn param_type_name(param: &str) -> Option<&str> {
    let ty = param.rsplit(':').next()?.trim();
    let ty = ty.split('<').next()?.trim();
    if ty.is_empty() {
        return None;
    }
    Some(ty)
}

fn param_is_asref_borrow_type_param(param: &str, asref_borrow_type_params: &[String]) -> bool {
    param_type_name(param).is_some_and(|ty| asref_borrow_type_params.iter().any(|p| p == ty))
}

fn param_is_asref_path_type_param(param: &str, asref_path_type_params: &[String]) -> bool {
    param_type_name(param).is_some_and(|ty| asref_path_type_params.iter().any(|p| p == ty))
}

fn param_has_asref_borrow_contract(param: &str, asref_borrow_type_params: &[String]) -> bool {
    // Signature-driven: any `AsRef<_>` (Path, str, [u8], …) or generic with that bound.
    param.contains("AsRef<") || param_is_asref_borrow_type_param(param, asref_borrow_type_params)
}

fn param_has_asref_path_contract(param: &str, asref_path_type_params: &[String]) -> bool {
    asref_target_type_name(param).is_some_and(is_asref_path_target)
        || param_is_asref_path_type_param(param, asref_path_type_params)
}

fn parse_parameters(params_str: &str, asref_borrow_type_params: &[String]) -> Vec<OwnershipMode> {
    if params_str.trim().is_empty() {
        return Vec::new();
    }

    split_top_level_commas(params_str)
        .into_iter()
        .map(|param| {
            let param = param.trim();

            // Check for &mut
            if param.contains("&mut ") {
                OwnershipMode::MutBorrowed
            }
            // AsRef<_> and generic params with those bounds
            else if param_has_asref_borrow_contract(param, asref_borrow_type_params) {
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

/// True when the scanned Rust formal lowers to shared borrow (`&str`, `&T`, `AsRef<_>`).
fn parse_emitted_rust_ref_flags(
    params_str: &str,
    asref_borrow_type_params: &[String],
) -> Option<Vec<bool>> {
    if params_str.trim().is_empty() {
        return Some(vec![]);
    }
    Some(
        split_top_level_commas(params_str)
            .into_iter()
            .map(|param| {
                let param = param.trim();
                param_has_asref_borrow_contract(param, asref_borrow_type_params)
                    || param.contains("&str")
                    || (param.contains('&') && !param.contains("&mut"))
            })
            .collect(),
    )
}

/// Best-effort Rust formal → WJ `Type` for IR/call-site coercion (signature-driven).
///
/// Maps `&str` / `AsRef<str>` to `Reference(str)`, `AsRef<Path>` to `Reference(Path)`,
/// and common owned formals (`String`, `Vec<String>`, …) to owned types.
fn parse_param_types(
    params_str: &str,
    asref_borrow_type_params: &[String],
    asref_path_type_params: &[String],
) -> Vec<Type> {
    if params_str.trim().is_empty() {
        return Vec::new();
    }
    split_top_level_commas(params_str)
        .into_iter()
        .map(|param| {
            parse_one_rust_param_type(
                param.trim(),
                asref_borrow_type_params,
                asref_path_type_params,
            )
        })
        .collect()
}

fn parse_one_rust_param_type(
    param: &str,
    asref_borrow_type_params: &[String],
    asref_path_type_params: &[String],
) -> Type {
    let ty = param.rsplit(':').next().unwrap_or(param).trim();

    if ty == "self" || ty == "&self" || ty.starts_with("&self") {
        return Type::Reference(Box::new(Type::Custom("Self".into())));
    }
    if ty == "&mut self" || ty.starts_with("&mut self") {
        return Type::MutableReference(Box::new(Type::Custom("Self".into())));
    }
    if ty == "mut self" || ty.starts_with("mut self") {
        return Type::Custom("Self".into());
    }

    // Distinguish Path vs str AsRef so formals can demote only for Path reuse
    // (`fs::write` → `fs::read`) while `strings::` / `db::` keep owned WJ `string`.
    if param_has_asref_path_contract(param, asref_path_type_params) {
        return Type::Reference(Box::new(Type::Custom("Path".into())));
    }
    if param_has_asref_borrow_contract(param, asref_borrow_type_params)
        || ty == "&str"
        || ty.starts_with("&str")
    {
        return Type::Reference(Box::new(Type::Custom("str".into())));
    }

    if let Some(inner) = ty.strip_prefix("&mut ") {
        return Type::MutableReference(Box::new(parse_owned_rust_type_name(inner.trim())));
    }
    if let Some(inner) = ty.strip_prefix('&') {
        return Type::Reference(Box::new(parse_owned_rust_type_name(inner.trim())));
    }
    if let Some(inner) = ty.strip_prefix("impl ") {
        if param_has_asref_path_contract(inner, &[]) {
            return Type::Reference(Box::new(Type::Custom("Path".into())));
        }
        if param_has_asref_borrow_contract(inner, &[]) {
            return Type::Reference(Box::new(Type::Custom("str".into())));
        }
        return parse_owned_rust_type_name(inner.trim());
    }
    parse_owned_rust_type_name(ty)
}

fn parse_return_type_after_params(after_params: &str) -> Option<Type> {
    let rest = after_params.trim();
    let rest = rest.strip_prefix("->")?.trim();
    let ty = rest
        .split('{')
        .next()
        .unwrap_or(rest)
        .split("where")
        .next()
        .unwrap_or(rest)
        .trim();
    if ty.is_empty() || ty == "!" {
        return None;
    }
    Some(parse_owned_rust_type_name(ty))
}

fn parse_generic_args(ty: &str) -> Option<Vec<Type>> {
    let start = ty.find('<')?;
    let inner = ty[start + 1..].strip_suffix('>')?;
    let mut args = Vec::new();
    let mut depth = 0i32;
    let mut start_arg = 0;
    for (i, c) in inner.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => {
                args.push(parse_owned_rust_type_name(inner[start_arg..i].trim()));
                start_arg = i + 1;
            }
            _ => {}
        }
    }
    let last = inner[start_arg..].trim();
    if !last.is_empty() {
        args.push(parse_owned_rust_type_name(last));
    }
    Some(args)
}

fn parse_owned_rust_type_name(ty: &str) -> Type {
    let ty = ty.trim();
    if ty == "()" {
        return Type::Tuple(vec![]);
    }
    // `&[T]` arrives here as `[T]` after the shared-ref `&` strip in `parse_one_rust_param_type`.
    // Represent slices as `Vec<T>` so prefer_shared_ref / call-site borrow match WJ `Vec`.
    if let Some(inner) = ty.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        return Type::Vec(Box::new(parse_owned_rust_type_name(inner.trim())));
    }
    let base = ty.split('<').next().unwrap_or(ty).trim();
    match base {
        "String" | "str" => Type::String,
        "bool" => Type::Bool,
        "i32" | "i64" | "isize" | "u32" | "u64" | "usize" | "f32" | "f64" => {
            Type::Custom(base.to_string())
        }
        "Vec" => {
            if let Some(inner) = ty.strip_prefix("Vec<").and_then(|s| s.strip_suffix('>')) {
                Type::Vec(Box::new(parse_owned_rust_type_name(inner.trim())))
            } else {
                Type::Vec(Box::new(Type::Custom("Unknown".into())))
            }
        }
        "Result" => {
            let args = parse_generic_args(ty).unwrap_or_default();
            match args.as_slice() {
                [ok, err] => Type::Result(Box::new(ok.clone()), Box::new(err.clone())),
                [ok] => Type::Result(Box::new(ok.clone()), Box::new(Type::String)),
                _ => Type::Custom("Result".into()),
            }
        }
        _ => Type::Custom(base.to_string()),
    }
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
        string_ref_string_formal_params: None,
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
            string_ref_string_formal_params: None,
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
            string_ref_string_formal_params: None,
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
            string_ref_string_formal_params: None,
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
            string_ref_string_formal_params: None,
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
            string_ref_string_formal_params: None,
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

    // strings::join(parts: &[String], delimiter: &str) — must stay Borrowed so WJ
    // `Vec<string>` args become `&parts` / bare `&Vec` deref, never `parts.clone()`.
    registry.add_function(
        "strings::join".to_string(),
        FunctionSignature {
            name: "strings::join".to_string(),
            param_types: vec![
                Type::Reference(Box::new(Type::Vec(Box::new(Type::String)))),
                Type::Reference(Box::new(Type::Custom("str".into()))),
            ],
            formal_param_types: vec![],
            param_ownership: vec![Borrowed, Borrowed],
            return_type: Some(Type::String),
            return_ownership: Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: Some(vec![true, true]),
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        },
    );

    register_wj_std_module_names(registry);
    register_runtime_modules_from_signature_keys(registry);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_exit_scan_registers_module_path() {
        let line = "pub fn exit(code: i32) -> ! {";
        let sig = parse_function_signature(line, "process").unwrap();
        assert_eq!(sig.name, "process::exit");
        assert_eq!(sig.param_ownership, vec![OwnershipMode::Owned]);
    }

    #[test]
    fn fs_write_scan_records_result_unit_return_type() {
        let line =
            "pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<(), String> {";
        let sig = parse_function_signature(line, "fs").unwrap();
        assert!(
            matches!(
                &sig.return_type,
                Some(Type::Result(ok, err))
                    if matches!(**ok, Type::Tuple(ref el) if el.is_empty())
                        && matches!(**err, Type::String)
            ),
            "fs::write must scan Result<(), String>, got {:?}",
            sig.return_type
        );
    }

    #[test]
    fn csv_wj_name_maps_to_csv_mod_rust_stem() {
        let stem = crate::analyzer::SignatureRegistry::stdlib().runtime_rust_stem("csv");
        assert_eq!(
            stem,
            Some("csv_mod"),
            "WJ std::csv must map to scanned rust stem csv_mod, got {stem:?}"
        );
        assert_eq!(
            crate::analyzer::SignatureRegistry::stdlib().runtime_rust_stem("async"),
            Some("async_runtime")
        );
        let regex_types =
            crate::analyzer::SignatureRegistry::stdlib().runtime_exported_types_for_module("regex");
        assert!(
            regex_types.iter().any(|t| *t == "Regex"),
            "regex_mod pub use Regex must be a scanned export, got {regex_types:?}"
        );
        let json_types =
            crate::analyzer::SignatureRegistry::stdlib().runtime_exported_types_for_module("json");
        assert!(
            json_types.iter().any(|t| *t == "Value"),
            "json pub use Value must be a scanned export, got {json_types:?}"
        );
        let http_types =
            crate::analyzer::SignatureRegistry::stdlib().runtime_exported_types_for_module("http");
        assert!(
            http_types.iter().any(|t| *t == "Response"),
            "http pub struct Response must be a scanned export, got {http_types:?}"
        );
    }

    #[test]
    fn http_runtime_types_map_to_windjammer_runtime_http() {
        let registry = crate::analyzer::SignatureRegistry::stdlib();
        for ty in ["HttpMethod", "ServerRequest", "ServerResponse"] {
            assert_eq!(
                registry.runtime_module_for_type(ty),
                Some("http"),
                "{ty} must scan to std::http, not a sibling module"
            );
            let expected = format!("windjammer_runtime::http::{ty}");
            assert_eq!(
                registry.runtime_rust_path_for_type(ty).as_deref(),
                Some(expected.as_str()),
                "{ty} FQ path"
            );
        }
    }

    #[test]
    fn scanned_runtime_modules_are_not_a_hardcoded_name_list() {
        use crate::codegen::rust::stdlib_method_traits::{
            is_runtime_std_module, runtime_std_module_for_type,
        };

        // File stems from windjammer-runtime/src — including modules historically
        // omitted from the hand-maintained list (`process`, `io`, `path`, `log`).
        for module in [
            "process", "io", "path", "http", "strings", "db", "fs", "log", "csv",
        ] {
            assert!(
                is_runtime_std_module(module),
                "{module} must be a scanned runtime std module"
            );
        }
        // WJ `use std::async` aliases `async_runtime.rs` (`_runtime` / std/async.wj).
        assert!(
            is_runtime_std_module("async") && is_runtime_std_module("async_runtime"),
            "async / async_runtime must both be runtime modules"
        );
        // `impl Connection` / `impl Row` in db.rs — not a hardcoded type table.
        assert_eq!(runtime_std_module_for_type("Connection"), Some("db"));
        assert_eq!(runtime_std_module_for_type("Row"), Some("db"));
        let req_fields =
            crate::analyzer::SignatureRegistry::stdlib().runtime_type_fields("ServerRequest");
        assert!(
            req_fields.iter().any(|(n, _)| n == "query")
                && req_fields.iter().any(|(n, _)| n == "body")
                && req_fields.iter().any(|(n, _)| n == "headers"),
            "ServerRequest fields must be scanned, got {req_fields:?}"
        );
        assert!(
            crate::analyzer::SignatureRegistry::stdlib().is_taint_sanitizer("regex::escape")
                || crate::analyzer::SignatureRegistry::stdlib()
                    .is_taint_sanitizer("regex_mod::escape"),
            "regex escape must be a scanned wj-taint sanitizer"
        );
        // User / unknown names must not match.
        assert!(!is_runtime_std_module("harness"));
        assert!(!is_runtime_std_module("server"));
        assert!(runtime_std_module_for_type("OptEconLedger").is_none());
        use crate::codegen::rust::stdlib_method_traits::{classify_wj_std_import, WjStdImportKind};
        assert!(
            matches!(
                classify_wj_std_import("csv"),
                WjStdImportKind::Runtime { rust_stem } if rust_stem == "csv_mod"
            ),
            "csv import must use scanned csv_mod stem"
        );
        assert!(matches!(
            classify_wj_std_import("process"),
            WjStdImportKind::Runtime { rust_stem } if rust_stem == "process"
        ));
        assert!(matches!(
            classify_wj_std_import("collections"),
            WjStdImportKind::RustStd
        ));
        assert!(
            matches!(classify_wj_std_import("fmt"), WjStdImportKind::RustStd),
            "rustc std modules without a runtime .rs file stay `use std::fmt`"
        );
        assert!(
            matches!(classify_wj_std_import("dialog"), WjStdImportKind::Skip),
            "WJ-only std/*.wj with no runtime file must not emit windjammer_runtime::dialog"
        );
    }

    #[test]
    fn resolve_runtime_src_finds_compiler_runtime_not_cwd() {
        let resolved = resolve_runtime_src_for_scan();
        assert!(
            resolved.as_ref().is_some_and(|p| p.join("strings.rs").exists()),
            "must locate windjammer-runtime/src/strings.rs via compiler install path, got {resolved:?}"
        );
    }

    #[test]
    fn parse_slice_param_as_reference_vec() {
        let ty = parse_one_rust_param_type("parts: &[String]", &[], &[]);
        assert!(
            matches!(
                ty,
                Type::Reference(ref inner) if matches!(**inner, Type::Vec(ref v) if matches!(**v, Type::String))
            ),
            "expected Reference(Vec(String)), got {ty:?}"
        );
    }

    #[test]
    fn strings_join_scan_marks_parts_borrowed_slice() {
        let line = "pub fn join(parts: &[String], delimiter: &str) -> String {";
        let sig = parse_function_signature(line, "strings").unwrap();
        assert_eq!(sig.param_ownership[0], OwnershipMode::Borrowed);
        assert_eq!(sig.param_ownership[1], OwnershipMode::Borrowed);
        assert_eq!(sig.emitted_rust_ref_params, Some(vec![true, true]));
        assert!(
            matches!(
                &sig.param_types[0],
                Type::Reference(inner) if matches!(**inner, Type::Vec(ref v) if matches!(**v, Type::String))
            ),
            "join parts must be Reference(Vec(String)), got {:?}",
            sig.param_types[0]
        );
    }

    #[test]
    fn fs_write_asref_path_marks_path_borrowed() {
        let line =
            "pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<(), String> {";
        let sig = parse_function_signature(line, "fs").unwrap();
        assert_eq!(sig.param_ownership[0], OwnershipMode::Borrowed);
        assert_eq!(sig.param_ownership[1], OwnershipMode::Borrowed);
        assert_eq!(sig.emitted_rust_ref_params, Some(vec![true, true]));
        assert!(
            matches!(
                &sig.param_types[0],
                Type::Reference(inner) if matches!(**inner, Type::Custom(ref n) if n == "Path")
            ),
            "AsRef<Path> must map to Reference(Path), got {:?}",
            sig.param_types[0]
        );
        assert!(
            matches!(
                &sig.param_types[1],
                Type::Reference(inner) if matches!(**inner, Type::Custom(ref n) if n == "str")
            ),
            "AsRef<[u8]> still maps to shared-ref str contract for WJ string, got {:?}",
            sig.param_types[1]
        );
        assert!(
            crate::codegen::rust::stdlib_method_traits::runtime_wj_owned_rust_borrowed_param(
                &sig, 0
            )
        );
    }

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
        assert_eq!(sig.param_ownership[0], OwnershipMode::Borrowed);
        assert_eq!(sig.param_ownership[1], OwnershipMode::Borrowed);
        assert!(
            matches!(
                &sig.param_types[0],
                Type::Reference(inner) if matches!(**inner, Type::Custom(ref n) if n == "str")
            ),
            "AsRef<str> must populate Reference(str), got {:?}",
            sig.param_types[0]
        );
        assert!(
            matches!(
                &sig.param_types[1],
                Type::Reference(inner) if matches!(**inner, Type::Custom(ref n) if n == "str")
            ),
            "&str must populate Reference(str), got {:?}",
            sig.param_types[1]
        );
    }

    #[test]
    fn connection_query_param_types_mark_sql_as_str_ref() {
        let line =
            "pub fn query(&self, sql: impl AsRef<str>, params: Vec<String>) -> Result<Vec<Row>, String> {";
        let sig = parse_function_signature(line, "db").unwrap();
        assert!(matches!(
            &sig.param_types[1],
            Type::Reference(inner) if matches!(**inner, Type::Custom(ref n) if n == "str")
        ));
        assert!(matches!(&sig.param_types[2], Type::Vec(inner) if matches!(**inner, Type::String)));
    }

    #[test]
    fn strings_len_asref_generic_param_is_borrowed() {
        let line = "pub fn len<S: AsRef<str>>(s: S) -> usize {";
        let sig = parse_function_signature(line, "strings").unwrap();
        assert_eq!(sig.name, "strings::len");
        assert_eq!(sig.param_ownership, vec![OwnershipMode::Borrowed]);
        assert_eq!(sig.emitted_rust_ref_params, Some(vec![true]));
    }

    #[test]
    fn csv_mod_registers_short_csv_alias() {
        let mut reg = SignatureRegistry::new();
        let sig = parse_function_signature(
            "pub fn parse(input: &str) -> Result<Vec<Vec<String>>, String> {",
            "csv_mod",
        )
        .unwrap();
        register_scanned_runtime_signature(&mut reg, "csv_mod", sig, None, false);
        let aliased = reg
            .get_signature("csv::parse")
            .expect("csv_mod must alias to csv::parse");
        assert_eq!(aliased.param_ownership[0], OwnershipMode::Borrowed);
        assert!(
            crate::codegen::rust::stdlib_method_traits::runtime_wj_owned_rust_borrowed_param(
                aliased, 0
            )
        );
    }

    #[test]
    fn self_method_does_not_clobber_free_function_under_module_path() {
        let mut reg = SignatureRegistry::new();
        let free = parse_function_signature(
            "pub fn post(url: &str, body: &str) -> Result<Response, String> {",
            "http",
        )
        .unwrap();
        register_scanned_runtime_signature(&mut reg, "http", free, None, false);
        let method = parse_function_signature(
            "pub fn post<F>(self, path: &str, handler: F) -> Self {",
            "http",
        )
        .unwrap();
        register_scanned_runtime_signature(&mut reg, "http", method, Some("Router"), false);

        let module_sig = reg
            .get_signature("http::post")
            .expect("http::post free function");
        assert!(
            !module_sig.has_self_receiver,
            "module path must keep free function, got {:?}",
            module_sig.param_types
        );
        assert_eq!(module_sig.param_ownership.len(), 2);
        assert_eq!(
            module_sig.param_ownership,
            vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed]
        );

        let typed = reg
            .get_signature("Router::post")
            .expect("Router::post method");
        assert!(typed.has_self_receiver);
        assert_eq!(typed.param_ownership.len(), 3);
    }

    #[test]
    fn scanned_runtime_http_post_is_free_two_str_refs() {
        let mut reg = SignatureRegistry::new();
        populate_runtime_signatures(&mut reg).expect("scan runtime");
        let sig = reg
            .get_signature("http::post")
            .expect("scanned http::post");
        assert!(
            !sig.has_self_receiver,
            "Router::post must not clobber free http::post, got {:?}",
            sig.param_types
        );
        assert_eq!(sig.param_ownership.len(), 2);
        assert!(
            crate::codegen::rust::stdlib_method_traits::runtime_wj_owned_rust_borrowed_param(
                sig, 1
            ),
            "body must need auto-borrow"
        );
        assert!(
            reg.get_signature("Router::post").is_some_and(|s| s.has_self_receiver),
            "Router::post must still be registered under Type::method"
        );
    }
}
