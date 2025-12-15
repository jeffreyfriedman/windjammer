// Type Analysis Module
//
// This module provides functions to analyze types and determine their traits:
// - Copy, Clone, Debug
// - PartialEq, Eq, Hash
// - Default
//
// These functions are used by the derive inference system to automatically
// add trait implementations to structs and enums.

use crate::parser::{EnumVariant, EnumVariantData, Item, Program, StructDecl, StructField, Type};
use std::collections::HashSet;

/// Type analyzer with knowledge of which custom types support various traits
pub struct TypeAnalyzer {
    /// Custom types (struct/enum names) that support PartialEq
    partial_eq_types: HashSet<String>,
}

impl TypeAnalyzer {
    /// Create a new TypeAnalyzer
    pub fn new() -> Self {
        Self {
            partial_eq_types: HashSet::new(),
        }
    }

    /// Pre-pass: Collect which custom types (structs/enums) support PartialEq
    /// This enables smart enum derives that only add PartialEq if all variants support it
    pub fn collect_partial_eq_types(&mut self, program: &Program) {
        for item in &program.items {
            match item {
                Item::Struct { decl: s, .. } => {
                    // Struct supports PartialEq if all fields do
                    if self.all_fields_are_partial_eq(&s.fields) {
                        self.partial_eq_types.insert(s.name.clone());
                    }
                }
                Item::Enum { decl: e, .. } => {
                    // Enum supports PartialEq if all variants do
                    if self.all_enum_variants_are_partial_eq(&e.variants) {
                        self.partial_eq_types.insert(e.name.clone());
                    }
                }
                _ => {}
            }
        }
    }

    // =============================================================================
    // Trait Derivation
    // =============================================================================

    /// Infer which traits can be safely derived for a struct
    pub fn infer_derivable_traits(&self, struct_: &StructDecl) -> Vec<String> {
        let mut traits = vec!["Debug".to_string(), "Clone".to_string()]; // Always safe to derive

        // Check if all fields are Copy
        if self.all_fields_are_copy(&struct_.fields) {
            traits.push("Copy".to_string());
        }

        // Check if all fields are PartialEq (most types support this)
        if self.all_fields_are_partial_eq(&struct_.fields) {
            traits.push("PartialEq".to_string());

            // Only add Eq if all fields support it (not floats)
            if self.all_fields_are_eq(&struct_.fields) {
                traits.push("Eq".to_string());

                // If Eq, also check for Hash
                if self.all_fields_are_hashable(&struct_.fields) {
                    traits.push("Hash".to_string());
                }
            }
        }

        // Check if all fields have Default
        if self.all_fields_have_default(&struct_.fields) {
            traits.push("Default".to_string());
        }

        traits
    }

    // =============================================================================
    // Field-Level Checks
    // =============================================================================

    /// Check if all fields in a struct are Copy
    pub fn all_fields_are_copy(&self, fields: &[StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.is_copy_type(&field.field_type))
    }

    /// Check if all fields in a struct support PartialEq
    pub fn all_fields_are_partial_eq(&self, fields: &[StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.is_partial_eq_type(&field.field_type))
    }

    /// Check if all fields in a struct support Eq (no floats)
    pub fn all_fields_are_eq(&self, fields: &[StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.is_eq_type(&field.field_type))
    }

    /// Check if all fields in a struct are hashable
    pub fn all_fields_are_hashable(&self, fields: &[StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.is_hashable_type(&field.field_type))
    }

    /// Check if all fields in a struct have Default implementations
    pub fn all_fields_have_default(&self, fields: &[StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.has_default(&field.field_type))
    }

    // =============================================================================
    // Enum Variant Checks
    // =============================================================================

    /// Check if all variants in an enum support PartialEq
    pub fn all_enum_variants_are_partial_eq(&self, variants: &[EnumVariant]) -> bool {
        variants.iter().all(|variant| {
            match &variant.data {
                EnumVariantData::Unit => true, // Unit variants always support PartialEq
                EnumVariantData::Tuple(types) => types.iter().all(|ty| self.is_partial_eq_type(ty)),
                EnumVariantData::Struct(fields) => fields
                    .iter()
                    .all(|(_, field_type)| self.is_partial_eq_type(field_type)),
            }
        })
    }

    /// Check if all variants in an enum support PartialEq (recursive check)
    pub fn all_enum_variants_are_partial_eq_recursive(&self, variants: &[EnumVariant]) -> bool {
        variants.iter().all(|variant| match &variant.data {
            EnumVariantData::Unit => true,
            EnumVariantData::Tuple(types) => {
                types.iter().all(|ty| self.is_partial_eq_type_recursive(ty))
            }
            EnumVariantData::Struct(fields) => fields
                .iter()
                .all(|(_, field_type)| self.is_partial_eq_type_recursive(field_type)),
        })
    }

    // =============================================================================
    // Type Trait Checks
    // =============================================================================

    /// Check if a type implements Copy
    pub fn is_copy_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
            Type::Reference(_) => true,         // References are Copy
            Type::MutableReference(_) => false, // Mutable references are not Copy
            Type::Tuple(types) => types.iter().all(|t| self.is_copy_type(t)),
            Type::Custom(name) => {
                // Recognize common Rust primitive types by name
                matches!(
                    name.as_str(),
                    "i8" | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "isize"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "usize"
                        | "f32"
                        | "f64"
                        | "bool"
                        | "char"
                )
            }
            _ => false, // String, Vec, Option, Result, other Custom types are not Copy
        }
    }

    /// Check if a type implements PartialEq
    pub fn is_partial_eq_type(&self, ty: &Type) -> bool {
        // Most types support PartialEq including floats
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool | Type::String => true,
            // Handle Rust-style type names that aren't Windjammer keywords
            Type::Custom(name)
                if matches!(
                    name.as_str(),
                    "i8" | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "isize"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "usize"
                        | "f32"
                        | "f64"
                        | "bool"
                        | "char"
                        | "String"
                        | "str"
                ) =>
            {
                true
            }
            // Check if custom type was collected as PartialEq
            Type::Custom(name) => self.partial_eq_types.contains(name),
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.is_partial_eq_type(inner)
            }
            Type::Tuple(types) => types.iter().all(|t| self.is_partial_eq_type(t)),
            Type::Parameterized(base, args) => {
                // Common generic types that support PartialEq if their type params do
                if matches!(
                    base.as_str(),
                    "Vec" | "Option" | "Result" | "Box" | "Rc" | "Arc"
                ) {
                    args.iter().all(|arg| self.is_partial_eq_type(arg))
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Check if a type implements PartialEq (recursive check for nested custom types)
    pub fn is_partial_eq_type_recursive(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool | Type::String => true,
            Type::Custom(name)
                if matches!(
                    name.as_str(),
                    "i8" | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "isize"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "usize"
                        | "f32"
                        | "f64"
                        | "bool"
                        | "char"
                        | "String"
                        | "str"
                ) =>
            {
                true
            }
            Type::Custom(name) => self.partial_eq_types.contains(name),
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.is_partial_eq_type_recursive(inner)
            }
            Type::Tuple(types) => types.iter().all(|t| self.is_partial_eq_type_recursive(t)),
            Type::Parameterized(base, args) => {
                if matches!(
                    base.as_str(),
                    "Vec" | "Option" | "Result" | "Box" | "Rc" | "Arc"
                ) {
                    args.iter()
                        .all(|arg| self.is_partial_eq_type_recursive(arg))
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Check if a type implements Eq (like PartialEq but no floats)
    pub fn is_eq_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Bool | Type::String => true,
            Type::Float => false, // Floats don't implement Eq
            Type::Custom(name)
                if matches!(
                    name.as_str(),
                    "i8" | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "isize"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "usize"
                        | "bool"
                        | "char"
                        | "String"
                        | "str"
                ) =>
            {
                true
            }
            Type::Custom(name) if matches!(name.as_str(), "f32" | "f64") => false, // No Eq for floats
            Type::Custom(name) => self.partial_eq_types.contains(name), // Assume custom types with PartialEq also have Eq (conservative)
            Type::Reference(inner) | Type::MutableReference(inner) => self.is_eq_type(inner),
            Type::Tuple(types) => types.iter().all(|t| self.is_eq_type(t)),
            Type::Parameterized(base, args) => {
                if matches!(
                    base.as_str(),
                    "Vec" | "Option" | "Result" | "Box" | "Rc" | "Arc"
                ) {
                    args.iter().all(|arg| self.is_eq_type(arg))
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Check if a type implements Hash
    pub fn is_hashable_type(&self, ty: &Type) -> bool {
        // Hash requires Eq, so we can use similar logic but exclude types that don't support Hash
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Bool | Type::String => true,
            Type::Float => false, // Floats don't implement Hash
            Type::Reference(inner) => self.is_hashable_type(inner),
            Type::MutableReference(_) => false, // &mut T doesn't implement Hash
            _ => self.is_eq_type(ty),           // If it has Eq, it likely has Hash
        }
    }

    /// Check if a type has a Default implementation
    pub fn has_default(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool | Type::String => true,
            Type::Custom(name)
                if matches!(
                    name.as_str(),
                    "i8" | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "isize"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "usize"
                        | "f32"
                        | "f64"
                        | "bool"
                        | "char"
                        | "String"
                ) =>
            {
                true
            }
            Type::Tuple(types) => types.iter().all(|t| self.has_default(t)),
            Type::Parameterized(base, args) => {
                // Vec, Option, Box, etc. have Default if their type params do
                if matches!(base.as_str(), "Vec" | "Option" | "Box") {
                    args.iter().all(|arg| self.has_default(arg))
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

impl Default for TypeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Type;

    #[test]
    fn test_is_copy_type_primitives() {
        let analyzer = TypeAnalyzer::new();
        assert!(analyzer.is_copy_type(&Type::Int));
        assert!(analyzer.is_copy_type(&Type::Bool));
        assert!(analyzer.is_copy_type(&Type::Float));
    }

    #[test]
    fn test_is_copy_type_non_copy() {
        let analyzer = TypeAnalyzer::new();
        assert!(!analyzer.is_copy_type(&Type::String));
    }

    #[test]
    fn test_is_partial_eq_type_primitives() {
        let analyzer = TypeAnalyzer::new();
        assert!(analyzer.is_partial_eq_type(&Type::Int));
        assert!(analyzer.is_partial_eq_type(&Type::Bool));
        assert!(analyzer.is_partial_eq_type(&Type::Float)); // Floats support PartialEq
        assert!(analyzer.is_partial_eq_type(&Type::String));
    }

    #[test]
    fn test_is_eq_type_no_floats() {
        let analyzer = TypeAnalyzer::new();
        assert!(analyzer.is_eq_type(&Type::Int));
        assert!(analyzer.is_eq_type(&Type::Bool));
        assert!(!analyzer.is_eq_type(&Type::Float)); // Floats don't support Eq
    }

    #[test]
    fn test_has_default() {
        let analyzer = TypeAnalyzer::new();
        assert!(analyzer.has_default(&Type::Int));
        assert!(analyzer.has_default(&Type::Bool));
        assert!(analyzer.has_default(&Type::String));
    }
}

// =============================================================================
// Pure Type Checking Functions
// =============================================================================

/// Check if a type implements Copy trait
///
/// Returns true for primitive types, references, and tuples of Copy types.
///
/// # Examples
/// ```
/// // i32, bool, f64 → true
/// // &T → true
/// // &mut T → false
/// // String, Vec<T> → false
/// ```
pub fn is_copy_type(ty: &Type) -> bool {
    match ty {
        Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
        Type::Reference(_) => true,         // References are Copy
        Type::MutableReference(_) => false, // Mutable references are not Copy
        Type::Tuple(types) => types.iter().all(is_copy_type),
        Type::Custom(name) => {
            // Recognize common Rust primitive types by name
            matches!(
                name.as_str(),
                "i8" | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "isize"
                    | "u8"
                    | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "usize"
                    | "f32"
                    | "f64"
                    | "bool"
                    | "char"
            )
        }
        _ => false, // String, Vec, Option, Result, other Custom types are not Copy
    }
}
