//! Interface Definition Language (IDL) representation for FFI generation.
//!
//! Built from analyzed `ModuleMetadata` so C headers and SDK bindings share one source of truth.

use crate::metadata::{FunctionSignature, ModuleMetadata};
use crate::parser::Type;

/// A Windjammer module exported through C FFI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdlModule {
    pub name: String,
    pub functions: Vec<IdlFunction>,
    pub types: Vec<IdlType>,
}

/// A callable entry in the IDL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdlFunction {
    pub name: String,
    pub params: Vec<IdlParam>,
    pub return_type: Option<String>,
    pub is_async: bool,
}

/// A function parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdlParam {
    pub name: String,
    pub type_name: String,
}

/// A struct, enum, or opaque type exposed to C.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdlType {
    pub name: String,
    pub kind: IdlTypeKind,
    pub fields: Vec<IdlField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdlTypeKind {
    Struct,
    Enum,
    Opaque,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdlField {
    pub name: String,
    pub type_name: String,
}

impl IdlModule {
    /// Build IDL from post-analysis module metadata (`.wj.meta` / multipass output).
    pub fn from_module_metadata(meta: &ModuleMetadata) -> Self {
        let module_name = if meta.module_path.is_empty() {
            "windjammer".to_string()
        } else {
            meta.module_path.clone()
        };

        let types = meta
            .structs
            .iter()
            .map(|(name, fields)| IdlType {
                name: name.clone(),
                kind: IdlTypeKind::Struct,
                fields: fields
                    .iter()
                    .map(|(field_name, type_str)| IdlField {
                        name: field_name.clone(),
                        type_name: wj_type_name_from_metadata(type_str),
                    })
                    .collect(),
            })
            .collect();

        let functions = meta
            .functions
            .iter()
            .filter_map(|(name, sig)| idl_function_from_metadata(name, sig))
            .collect();

        Self {
            name: module_name,
            functions,
            types,
        }
    }
}

fn idl_function_from_metadata(name: &str, sig: &FunctionSignature) -> Option<IdlFunction> {
    if sig.has_self_receiver {
        return None;
    }

    let param_types = if sig.formal_params.is_empty() {
        &sig.params
    } else {
        &sig.formal_params
    };

    let ownership = &sig.param_ownership;
    let mut params = Vec::with_capacity(param_types.len());
    for (idx, type_str) in param_types.iter().enumerate() {
        let type_name = wj_type_name_from_metadata(type_str);
        let borrowed = ownership
            .get(idx)
            .is_some_and(|o| o == "Borrowed" || o == "MutBorrowed");
        let c_type = if borrowed && is_string_type(type_str) {
            "const char*".to_string()
        } else {
            type_name
        };
        params.push(IdlParam {
            name: format!("param{}", idx + 1),
            type_name: c_type,
        });
    }

    let return_type = sig
        .return_type
        .as_ref()
        .map(|t| wj_type_name_from_metadata(t));

    let is_async = sig
        .return_type
        .as_deref()
        .is_some_and(|t| t.contains("Future") || t.contains("async"));

    Some(IdlFunction {
        name: export_function_name(name),
        params,
        return_type,
        is_async,
    })
}

fn export_function_name(qualified: &str) -> String {
    qualified
        .rsplit("::")
        .next()
        .unwrap_or(qualified)
        .to_string()
}

fn is_string_type(serialized: &str) -> bool {
    serialized == "String"
        || serialized == "string"
        || serialized == "Custom(\"string\")"
        || serialized.contains("string")
}

/// Map serialized metadata type strings to Windjammer/C-friendly names.
pub fn wj_type_name_from_metadata(serialized: &str) -> String {
    if let Some(ty) = ModuleMetadata::deserialize_type(serialized) {
        return wj_type_name_from_ast(&ty);
    }

    match serialized {
        "Int32" => "int32_t".to_string(),
        "Float" => "float".to_string(),
        "Bool" => "bool".to_string(),
        "String" | "string" | "Custom(\"string\")" => "const char*".to_string(),
        s if s.starts_with("Custom(\"") && s.ends_with("\")") => {
            s.strip_prefix("Custom(\"")
                .and_then(|r| r.strip_suffix("\")"))
                .unwrap_or(s)
                .to_string()
        }
        other => other.to_string(),
    }
}

fn wj_type_name_from_ast(ty: &Type) -> String {
    use crate::codegen::rust::types::is_windjammer_text_type;
    if is_windjammer_text_type(ty) {
        return "const char*".to_string();
    }

    match ty {
        Type::Int32 => "int32_t".to_string(),
        Type::Float => "float".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Custom(s) if s == "i32" || s == "int" => "int32_t".to_string(),
        Type::Custom(s) if s == "f32" || s == "float" => "float".to_string(),
        Type::Custom(s) if s == "bool" => "bool".to_string(),
        Type::Custom(s) if s == "u32" => "uint32_t".to_string(),
        Type::Custom(s) if s == "u64" => "uint64_t".to_string(),
        Type::Custom(s) if s == "i64" => "int64_t".to_string(),
        Type::Custom(s) if s == "f64" || s == "double" => "double".to_string(),
        Type::Custom(s) if s == "usize" => "size_t".to_string(),
        Type::String => "const char*".to_string(),
        Type::Custom(name) => name.clone(),
        _ => serialized_fallback(ty),
    }
}

fn serialized_fallback(ty: &Type) -> String {
    ModuleMetadata::serialize_type(ty)
        .trim_start_matches("Custom(\"")
        .trim_end_matches("\")")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::FunctionSignature;
    use std::collections::HashMap;

    #[test]
    fn from_module_metadata_builds_structs_and_functions() {
        let mut meta = ModuleMetadata::new("math".to_string());
        let mut fields = HashMap::new();
        fields.insert("x".to_string(), "Custom(\"i32\")".to_string());
        fields.insert("y".to_string(), "Custom(\"f32\")".to_string());
        meta.structs.insert("MyStruct".to_string(), fields);

        meta.functions.insert(
            "my_function".to_string(),
            FunctionSignature {
                params: vec!["Custom(\"i32\")".to_string(), "String".to_string()],
                formal_params: vec![],
                return_type: Some("Custom(\"MyStruct\")".to_string()),
                is_associated: false,
                parent_type: None,
                param_ownership: vec!["Owned".to_string(), "Borrowed".to_string()],
                emitted_rust_ref_params: None,
                has_self_receiver: false,
                is_extern: true,
            },
        );

        let idl = IdlModule::from_module_metadata(&meta);
        assert_eq!(idl.name, "math");
        assert_eq!(idl.types.len(), 1);
        assert_eq!(idl.types[0].name, "MyStruct");
        assert_eq!(idl.types[0].fields.len(), 2);
        assert_eq!(idl.functions.len(), 1);
        assert_eq!(idl.functions[0].name, "my_function");
        assert_eq!(idl.functions[0].params.len(), 2);
        assert_eq!(idl.functions[0].params[1].type_name, "const char*");
    }

    #[test]
    fn skips_methods_with_self_receiver() {
        let mut meta = ModuleMetadata::new("game".to_string());
        meta.functions.insert(
            "Player::update".to_string(),
            FunctionSignature {
                params: vec![],
                formal_params: vec![],
                return_type: None,
                is_associated: true,
                parent_type: Some("Player".to_string()),
                param_ownership: vec![],
                emitted_rust_ref_params: None,
                has_self_receiver: true,
                is_extern: false,
            },
        );

        let idl = IdlModule::from_module_metadata(&meta);
        assert!(idl.functions.is_empty());
    }
}
