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
    
    /// Deserialize Type from JSON string
    pub fn deserialize_type(s: &str) -> Option<Type> {
        // For MVP: Parse simple types manually
        // TODO: Proper serde for Type enum
        match s {
            "Custom(\"f32\")" => Some(Type::Custom("f32".to_string())),
            "Custom(\"f64\")" => Some(Type::Custom("f64".to_string())),
            "Custom(\"i32\")" => Some(Type::Custom("i32".to_string())),
            "Int32" => Some(Type::Int32),
            "Float" => Some(Type::Float),
            _ => None, // TODO: Handle complex types
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
