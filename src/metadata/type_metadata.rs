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
        "Bool" | "Int32" | "Float" => true,
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
            "Int32" => Some(Type::Int32),
            "Float" => Some(Type::Float),
            "Bool" => Some(Type::Bool),
            "String" => Some(Type::String),
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
            s if s.starts_with("Custom(") => {
                // Custom("TypeName") - extract the inner string
                let rest = s
                    .strip_prefix("Custom(\"")
                    .and_then(|r| r.strip_suffix("\")"));
                rest.map(|name| Type::Custom(name.to_string()))
            }
            _ => None,
        }
    }
}
