/// Metadata System for Cross-Module Type Inference
///
/// Enables type inference across file boundaries by emitting and loading
/// function signatures, struct fields, and trait implementations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::parser::ast::types::Type;

/// Metadata for a single Windjammer module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetadata {
    /// Module path (e.g., "math::vec3")
    pub module_path: String,
    
    /// Function signatures: name → (param_types, return_type)
    pub functions: HashMap<String, FunctionSignature>,
    
    /// Struct field types: struct_name → field_name → Type
    pub structs: HashMap<String, HashMap<String, String>>, // String = serialized Type
    
    /// Trait implementations: trait_name → methods
    pub trait_impls: HashMap<String, Vec<String>>,
    
    /// Version for compatibility checking
    pub version: String,
}

/// Function signature for type inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    /// Parameter types
    pub params: Vec<String>, // Serialized Type (JSON is easier than bincode for now)
    
    /// Return type (None for unit)
    pub return_type: Option<String>,
    
    /// Is this an associated function (Type::method)?
    pub is_associated: bool,
    
    /// Parent type for associated functions (e.g., "Vec3" for Vec3::new)
    pub parent_type: Option<String>,
}

/// Crate-level metadata for cross-crate type inference.
/// Emitted as metadata.json when building libraries (--library).
/// Loaded when compiling apps that depend on external Windjammer crates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateMetadata {
    /// All structs: struct_name → field_name → serialized Type
    pub structs: HashMap<String, HashMap<String, String>>,
    /// All function signatures: name → signature
    pub functions: HashMap<String, FunctionSignature>,
    /// Version for compatibility
    pub version: String,
}

impl Default for CrateMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl CrateMetadata {
    pub fn new() -> Self {
        CrateMetadata {
            structs: HashMap::new(),
            functions: HashMap::new(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Merge in struct/function data from a ModuleMetadata
    pub fn merge_module(&mut self, module: &ModuleMetadata) {
        for (struct_name, fields) in &module.structs {
            self.structs
                .entry(struct_name.clone())
                .or_default()
                .extend(fields.clone());
        }
        for (func_name, sig) in &module.functions {
            self.functions.insert(func_name.clone(), sig.clone());
        }
    }
}

impl ModuleMetadata {
    pub fn new(module_path: String) -> Self {
        ModuleMetadata {
            module_path,
            functions: HashMap::new(),
            structs: HashMap::new(),
            trait_impls: HashMap::new(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
    
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
            s if s.starts_with("Custom(") => {
                // Custom("TypeName") - extract the inner string
                let rest = s.strip_prefix("Custom(\"").and_then(|r| r.strip_suffix("\")"));
                rest.map(|name| Type::Custom(name.to_string()))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_round_trip() {
        let mut meta = ModuleMetadata::new("math::vec3".to_string());
        
        meta.functions.insert(
            "Vec3::new".to_string(),
            FunctionSignature {
                params: vec![
                    "Custom(\"f32\")".to_string(),
                    "Custom(\"f32\")".to_string(),
                    "Custom(\"f32\")".to_string(),
                ],
                return_type: Some("Custom(\"Vec3\")".to_string()),
                is_associated: true,
                parent_type: Some("Vec3".to_string()),
            },
        );
        
        let json = serde_json::to_string_pretty(&meta).unwrap();
        eprintln!("Metadata JSON:\n{}", json);
        
        let loaded: ModuleMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.functions.len(), 1);
        assert!(loaded.functions.contains_key("Vec3::new"));
    }
}
