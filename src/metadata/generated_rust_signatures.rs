//! Recover analyzer signatures from generated Rust when `metadata.json` is missing.
//!
//! Ecosystem packages often ship `build/lib.rs` (real `&str` / `&T` formals) without
//! `--library` metadata. Call sites must not invent ownership from the callee name.

use crate::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use crate::parser::Type;
use std::path::Path;

/// Register `crate_key::fn` signatures recovered from generated `*.rs` in `dir`.
pub(in crate::metadata) fn merge_generated_rust_dir_with_alias(
    dir: &Path,
    registry: &mut SignatureRegistry,
    crate_alias: Option<&str>,
) {
    for (name, sig) in recover_signatures_from_generated_rust_dir(dir) {
        if let Some(alias) = crate_alias {
            if !alias.is_empty() && !name.contains("::") {
                let qualified = format!("{alias}::{name}");
                // Generated `lib.rs` is the published ABI when `metadata.json` is
                // missing. Stale `.wj.meta` / analyzer stubs (`emitted [false, false]`)
                // must not hide `&str` formals.
                registry.add_function(qualified, sig);
                continue;
            }
        }
        registry.add_function(name, sig);
    }
}

/// Parse `pub fn` formals from `lib.rs` and sibling `.rs` files (not recursive).
pub fn recover_signatures_from_generated_rust_dir(dir: &Path) -> Vec<(String, FunctionSignature)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .collect();
    files.sort();
    for path in files {
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        out.extend(recover_signatures_from_generated_rust_source(&src));
    }
    out
}

pub fn recover_signatures_from_generated_rust_source(
    src: &str,
) -> Vec<(String, FunctionSignature)> {
    let mut out = Vec::new();
    let mut search = src;
    let mut offset = 0usize;
    while let Some(rel) = search.find("fn ") {
        let abs = offset + rel;
        let before = src[..abs].trim_end();
        if !before.ends_with("pub") && !before.ends_with("pub ") {
            offset = abs + 3;
            search = &src[offset..];
            continue;
        }
        let after_fn = &src[abs + 3..];
        let name_end = after_fn
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(after_fn.len());
        let name = after_fn[..name_end].trim();
        if name.is_empty() {
            offset = abs + 3;
            search = &src[offset..];
            continue;
        }
        let rest = after_fn[name_end..].trim_start();
        let rest = if rest.starts_with('<') {
            match skip_balanced(rest, '<', '>') {
                Some(i) => rest[i + 1..].trim_start(),
                None => {
                    offset = abs + 3;
                    search = &src[offset..];
                    continue;
                }
            }
        } else {
            rest
        };
        if !rest.starts_with('(') {
            offset = abs + 3;
            search = &src[offset..];
            continue;
        }
        let Some(close) = find_matching(rest, '(', ')') else {
            offset = abs + 3;
            search = &src[offset..];
            continue;
        };
        let params_src = &rest[1..close];
        let after_params = rest[close + 1..].trim_start();
        let return_type = parse_rust_return_type(after_params);
        if let Some(sig) = signature_from_rust_params(name, params_src, return_type) {
            out.push((name.to_string(), sig));
        }
        offset = abs + 3 + name_end + close;
        search = &src[offset..];
    }
    out
}

/// Parse `-> T` after formals; stop before `{` / `;` / where-clause noise.
fn parse_rust_return_type(after_params: &str) -> Option<Type> {
    let t = after_params.trim_start();
    let t = t.strip_prefix("->")?;
    let t = t.trim_start();
    let end = t
        .find(|c: char| c == '{' || c == ';' || c == '\n')
        .unwrap_or(t.len());
    let ty_src = t[..end].trim();
    if ty_src.is_empty() {
        return None;
    }
    Some(parse_rust_type(ty_src))
}

fn signature_from_rust_params(
    name: &str,
    params_src: &str,
    return_type: Option<Type>,
) -> Option<FunctionSignature> {
    let mut param_types = Vec::new();
    let mut param_ownership = Vec::new();
    let mut emitted = Vec::new();
    let mut has_self = false;
    for raw in split_top_level(params_src, ',') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        if is_self_param(raw) {
            has_self = true;
            continue;
        }
        let ty_src = raw
            .rsplit_once(':')
            .map(|(_, t)| t.trim())
            .unwrap_or(raw);
        let ty = parse_rust_type(ty_src);
        let (own, flag) = ownership_from_type(&ty);
        param_types.push(ty);
        param_ownership.push(own);
        emitted.push(flag);
    }
    if has_self {
        return None;
    }
    Some(FunctionSignature {
        name: name.to_string(),
        formal_param_types: param_types.clone(),
        param_types,
        param_ownership,
        return_type,
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
        emitted_rust_ref_params: Some(emitted),
        string_ref_string_formal_params: None,
        field_extract_params: None,
        forwarding_borrow_params: None,
    })
}

fn is_self_param(raw: &str) -> bool {
    matches!(
        raw.trim(),
        "self" | "&self" | "&mut self" | "mut self"
    ) || raw.trim_start().starts_with("self:")
}

fn ownership_from_type(ty: &Type) -> (OwnershipMode, bool) {
    match ty {
        Type::MutableReference(_) => (OwnershipMode::MutBorrowed, true),
        Type::Reference(_) => (OwnershipMode::Borrowed, true),
        _ => (OwnershipMode::Owned, false),
    }
}

fn parse_rust_type(raw: &str) -> Type {
    let t = raw.trim();
    if let Some(inner) = t.strip_prefix("&mut ") {
        return Type::MutableReference(Box::new(parse_rust_type(inner)));
    }
    if let Some(inner) = t.strip_prefix('&') {
        return Type::Reference(Box::new(parse_rust_type(inner.trim())));
    }
    match t {
        "str" | "String" => return Type::String,
        "bool" => return Type::Bool,
        "i32" | "int" => return Type::Int32,
        "i64" => return Type::Int,
        "u16" | "u32" | "u64" | "usize" | "uint" => return Type::Uint,
        "f32" | "f64" => return Type::Float,
        _ => {}
    }
    if let Some(inner) = strip_wrapper(t, "Option") {
        return Type::Option(Box::new(parse_rust_type(inner)));
    }
    if let Some(inner) = strip_wrapper(t, "Result") {
        let parts = split_top_level(inner, ',');
        if parts.len() == 2 {
            return Type::Result(
                Box::new(parse_rust_type(parts[0])),
                Box::new(parse_rust_type(parts[1])),
            );
        }
    }
    if let Some(inner) = strip_wrapper(t, "Vec") {
        return Type::Vec(Box::new(parse_rust_type(inner)));
    }
    // P3.537: path-dep `Result<HashMap<String, String>, …>` must type Ok(map) as
    // HashMap so `map.get("lit")` resolves to HashMap::get(&Q), not unresolved auto-own.
    for map_name in ["HashMap", "BTreeMap"] {
        if let Some(inner) = strip_wrapper(t, map_name) {
            let parts = split_top_level(inner, ',');
            if parts.len() == 2 {
                return Type::Parameterized(
                    map_name.to_string(),
                    vec![parse_rust_type(parts[0]), parse_rust_type(parts[1])],
                );
            }
        }
    }
    if t.starts_with('(') && t.ends_with(')') {
        let inner = &t[1..t.len() - 1];
        let elems: Vec<Type> = split_top_level(inner, ',')
            .into_iter()
            .filter(|p| !p.trim().is_empty())
            .map(parse_rust_type)
            .collect();
        return Type::Tuple(elems);
    }
    Type::Custom(t.to_string())
}

fn strip_wrapper<'a>(t: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}<");
    if t.starts_with(&prefix) && t.ends_with('>') {
        Some(&t[prefix.len()..t.len() - 1])
    } else {
        None
    }
}

fn split_top_level(src: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut angle = 0i32;
    let mut paren = 0i32;
    for (i, c) in src.char_indices() {
        match c {
            '<' => angle += 1,
            '>' => angle -= 1,
            '(' => paren += 1,
            ')' => paren -= 1,
            c if c == sep && angle == 0 && paren == 0 => {
                out.push(&src[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&src[start..]);
    out
}

fn find_matching(src: &str, open: char, close: char) -> Option<usize> {
    let mut depth = 0i32;
    for (i, c) in src.char_indices() {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn skip_balanced(src: &str, open: char, close: char) -> Option<usize> {
    find_matching(src, open, close)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_log_tagged_shared_str_formals() {
        let src = r#"
#[inline]
pub fn log_tagged(level: &str, tag: &str, message: &str) {}
"#;
        let recovered = recover_signatures_from_generated_rust_source(src);
        let sig = &recovered
            .iter()
            .find(|(n, _)| n == "log_tagged")
            .expect("log_tagged")
            .1;
        assert_eq!(
            sig.param_ownership,
            vec![
                OwnershipMode::Borrowed,
                OwnershipMode::Borrowed,
                OwnershipMode::Borrowed
            ]
        );
        assert_eq!(
            sig.emitted_rust_ref_params.as_deref(),
            Some(&[true, true, true][..])
        );
    }

    #[test]
    fn recovers_qs_get_mixed_owned_query_shared_key() {
        let src = "pub fn get(query: String, key: &str) -> Option<String> { None }";
        let recovered = recover_signatures_from_generated_rust_source(src);
        let sig = &recovered.iter().find(|(n, _)| n == "get").expect("get").1;
        assert_eq!(
            sig.param_ownership,
            vec![OwnershipMode::Owned, OwnershipMode::Borrowed]
        );
        assert_eq!(
            sig.emitted_rust_ref_params.as_deref(),
            Some(&[false, true][..])
        );
        assert!(
            matches!(sig.return_type, Some(Type::Option(_))),
            "return type must be recovered: {:?}",
            sig.return_type
        );
    }

    #[test]
    fn recovers_parse_cookie_header_result_hashmap_return() {
        let src = r#"
#[inline]
pub fn parse_cookie_header(header: &str) -> Result<HashMap<String, String>, String> {
    Ok(HashMap::new())
}
"#;
        let recovered = recover_signatures_from_generated_rust_source(src);
        let sig = &recovered
            .iter()
            .find(|(n, _)| n == "parse_cookie_header")
            .expect("parse_cookie_header")
            .1;
        assert_eq!(sig.param_ownership, vec![OwnershipMode::Borrowed]);
        match &sig.return_type {
            Some(Type::Result(ok, err)) => {
                assert!(
                    matches!(ok.as_ref(), Type::Parameterized(n, args)
                        if n == "HashMap" && args.len() == 2),
                    "Ok payload must be Parameterized HashMap, got {ok:?}"
                );
                assert!(matches!(err.as_ref(), Type::String), "Err must be String");
            }
            other => panic!("expected Result<HashMap, String>, got {other:?}"),
        }
    }
}
