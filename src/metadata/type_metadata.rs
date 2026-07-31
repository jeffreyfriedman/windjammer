//! Type serialization for metadata and Copy inference from struct field metadata.

use std::collections::{HashMap, HashSet};

use crate::parser::ast::types::Type;

use super::ModuleMetadata;

/// Public accessor for `infer_copy_from_metadata_structs` (used by compiler multipass).
pub fn infer_copy_from_metadata_structs_pub(
    all_struct_fields: &HashMap<String, Vec<Vec<String>>>,
    existing_copy: &mut Vec<String>,
) {
    infer_copy_from_metadata_structs(all_struct_fields, existing_copy);
}

/// Infer Copy types from struct field definitions loaded from metadata.
/// A struct is Copy if all its fields are known Copy types.
/// Uses fixpoint iteration to handle transitive Copy (e.g., struct A { b: B } where B is Copy).
///
/// TDD FIX: Conservative handling of duplicate struct names across modules.
/// If multiple metadata files define structs with the same name, only mark as Copy
/// if ALL variants are Copy. This prevents one Copy-able GameState from poisoning
/// a non-Copy GameState in a different module.
pub(in crate::metadata) fn infer_copy_from_metadata_structs(
    all_struct_fields: &HashMap<String, Vec<Vec<String>>>,
    existing_copy: &mut Vec<String>,
) {
    let mut copy_set: HashSet<String> = existing_copy.iter().cloned().collect();

    const MAX_PASSES: usize = 32;
    for _ in 0..MAX_PASSES {
        let mut changed = false;
        for (struct_name, variants) in all_struct_fields {
            if copy_set.contains(struct_name) {
                continue;
            }

            // TDD FIX: Check if ALL variants are Copy (conservative)
            let all_variants_copy = variants.iter().all(|field_types| {
                field_types
                    .iter()
                    .all(|ft| is_copy_type_string(ft, &copy_set))
            });

            if all_variants_copy {
                copy_set.insert(struct_name.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    for name in &copy_set {
        if !existing_copy.contains(name) {
            existing_copy.push(name.clone());
        }
    }
}

/// Check if a serialized Type string represents a Copy type.
fn is_copy_type_string(s: &str, copy_set: &HashSet<String>) -> bool {
    match s {
        "Bool" | "Int" | "Int32" | "Uint" | "Float" => true,
        s if s.starts_with("Custom(\"") && s.ends_with("\")") => {
            let name = &s[8..s.len() - 2];
            matches!(
                name,
                "f32"
                    | "f64"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "u8"
                    | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "usize"
                    | "isize"
                    | "bool"
                    | "char"
            ) || copy_set.contains(name)
        }
        s if s.starts_with("Array(") => {
            // Array(InnerType, N) - Copy if InnerType is Copy
            let inner = &s[6..s.len() - 1];
            if let Some(comma_pos) = inner.rfind(", ") {
                let ty_str = &inner[..comma_pos];
                is_copy_type_string(ty_str.trim(), copy_set)
            } else {
                false
            }
        }
        _ => false,
    }
}

impl ModuleMetadata {
    /// Serialize Type to JSON string (for metadata storage)
    pub fn serialize_type(ty: &Type) -> String {
        // For MVP: Use Debug format (simple but works)
        // TODO: Proper serde for Type enum
        format!("{:?}", ty)
    }

    /// Deserialize Type from JSON string (Debug format from serialize_type)
    pub fn deserialize_type(s: &str) -> Option<Type> {
        // For MVP: Parse simple types manually
        // TODO: Proper serde for Type enum
        match s {
            "Custom(\"f32\")" => Some(Type::Custom("f32".to_string())),
            "Custom(\"f64\")" => Some(Type::Custom("f64".to_string())),
            "Custom(\"i32\")" => Some(Type::Custom("i32".to_string())),
            "Custom(\"u32\")" => Some(Type::Custom("u32".to_string())),
            "Custom(\"Self\")" => Some(Type::Custom("Self".to_string())),
            "Int" => Some(Type::Int),
            "Int32" => Some(Type::Int32),
            "Uint" => Some(Type::Uint),
            "Float" => Some(Type::Float),
            "Bool" => Some(Type::Bool),
            "String" => Some(Type::String),
            "Infer" => Some(Type::Infer),
            "string" | "Custom(\"string\")" => Some(Type::String),
            s if s.starts_with("Array(") && s.ends_with(')') => {
                // Array(Custom("f32"), 16) or Array(InnerType, N)
                let inner = &s[6..s.len() - 1];
                if let Some(comma_pos) = inner.rfind(", ") {
                    let (ty_str, n_str) = inner.split_at(comma_pos);
                    let n_str = n_str.trim_start_matches(", ");
                    if let (Some(inner_ty), Ok(n)) = (
                        Self::deserialize_type(ty_str.trim()),
                        n_str.parse::<usize>(),
                    ) {
                        return Some(Type::Array(Box::new(inner_ty), n));
                    }
                }
                None
            }
            s if s.starts_with("Vec(") && s.ends_with(')') => {
                let inner = &s[4..s.len() - 1];
                Self::deserialize_type(inner).map(|t| Type::Vec(Box::new(t)))
            }
            s if s.starts_with("Option(") && s.ends_with(')') => {
                let inner = &s[7..s.len() - 1];
                Self::deserialize_type(inner).map(|t| Type::Option(Box::new(t)))
            }
            s if s.starts_with("Result(") && s.ends_with(')') => {
                let inner = &s[7..s.len() - 1];
                split_debug_type_list(inner).and_then(|parts| {
                    if parts.len() != 2 {
                        return None;
                    }
                    Some(Type::Result(
                        Box::new(Self::deserialize_type(&parts[0])?),
                        Box::new(Self::deserialize_type(&parts[1])?),
                    ))
                })
            }
            s if s.starts_with("Tuple(") && s.ends_with(')') => {
                let inner = &s[6..s.len() - 1];
                // Debug: Tuple([T1, T2, ...])
                let inner = inner
                    .strip_prefix('[')
                    .and_then(|r| r.strip_suffix(']'))
                    .unwrap_or(inner);
                let parts = split_debug_type_list(inner)?;
                let mut types = Vec::with_capacity(parts.len());
                for p in parts {
                    types.push(Self::deserialize_type(&p)?);
                }
                Some(Type::Tuple(types))
            }
            s if s.starts_with("Parameterized(") && s.ends_with(')') => {
                // Parameterized("HashMap", [String, String])
                let inner = &s["Parameterized(".len()..s.len() - 1];
                let (name, args_str) = split_parameterized_debug(inner)?;
                let arg_parts = split_debug_type_list(args_str)?;
                let mut args = Vec::with_capacity(arg_parts.len());
                for p in arg_parts {
                    args.push(Self::deserialize_type(&p)?);
                }
                Some(Type::Parameterized(name, args))
            }
            s if s.starts_with("Generic(\"") && s.ends_with("\")") => {
                let name = &s["Generic(\"".len()..s.len() - 2];
                Some(Type::Generic(name.to_string()))
            }
            s if s.starts_with("Reference(") && s.ends_with(')') => {
                let inner = &s[10..s.len() - 1];
                Self::deserialize_type(inner).map(|t| Type::Reference(Box::new(t)))
            }
            s if s.starts_with("MutableReference(") && s.ends_with(')') => {
                let inner = &s[17..s.len() - 1];
                Self::deserialize_type(inner).map(|t| Type::MutableReference(Box::new(t)))
            }
            s if s.starts_with("Custom(") => {
                let rest = s
                    .strip_prefix("Custom(\"")
                    .and_then(|r| r.strip_suffix("\")"));
                rest.map(|name| Type::Custom(name.to_string()))
            }
            _ => None,
        }
    }
}

/// Split `Parameterized("Name", [T1, T2])` inner into `(Name, "T1, T2")`.
fn split_parameterized_debug(inner: &str) -> Option<(String, &str)> {
    let rest = inner.strip_prefix('"')?;
    let name_end = rest.find('"')?;
    let name = rest[..name_end].to_string();
    let after_name = rest[name_end + 1..].trim_start();
    let after_comma = after_name.strip_prefix(',')?.trim_start();
    let args = after_comma
        .strip_prefix('[')?
        .strip_suffix(']')?;
    Some((name, args))
}

/// Split a Debug type-list like `String, Int` or `Parameterized("HashMap", [String, String]), Int`
/// respecting nested parentheses/brackets.
fn split_debug_type_list(s: &str) -> Option<Vec<String>> {
    if s.trim().is_empty() {
        return Some(Vec::new());
    }
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    let chars: Vec<char> = s.chars().collect();
    for (i, ch) in chars.iter().enumerate() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(chars[start..i].iter().collect::<String>().trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
        if depth < 0 {
            return None;
        }
    }
    if depth != 0 {
        return None;
    }
    parts.push(chars[start..].iter().collect::<String>().trim().to_string());
    Some(parts)
}
