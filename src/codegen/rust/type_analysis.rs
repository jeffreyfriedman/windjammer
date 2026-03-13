// Type Analysis Module
//
// This module provides functions to analyze types and determine their traits:
// - Copy, Clone, Debug
// - PartialEq, Eq, Hash
// - Default
//
// These functions are used by the derive inference system to automatically
// add trait implementations to structs and enums.
//
// Also contains type inference helper methods for CodeGenerator (infer_type_name,
// infer_expression_type, expression_produces_usize, etc.).

use crate::parser::{
    BinaryOp, EnumPatternBinding, EnumVariant, EnumVariantData, Expression, Item, Literal, Pattern,
    Program, Statement, StructDecl, StructField, Type,
};
use std::collections::HashSet;

use super::CodeGenerator;

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
            Type::Reference(_) => true,           // References are Copy
            Type::MutableReference(_) => false,   // Mutable references are not Copy
            Type::RawPointer { .. } => true,      // TDD: Raw pointers are Copy (like &T)
            Type::FunctionPointer { .. } => true, // TDD FIX: Function pointers are Copy!
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
        Type::Reference(_) => true,           // References are Copy
        Type::MutableReference(_) => false,   // Mutable references are not Copy
        Type::RawPointer { .. } => true,      // TDD: Raw pointers are Copy (like &T)
        Type::FunctionPointer { .. } => true, // TDD FIX: Function pointers are Copy!
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

/// Known Copy types from external crates (windjammer-app, game engines).
/// These are common structs with only primitive fields - always Copy in Rust.
/// Used when struct definition isn't available (cross-crate compilation).
/// NOTE: SaveSlot/SaveSlotInfo have String fields - NOT included.
pub fn is_known_copy_type(name: &str) -> bool {
    matches!(
        name,
        "Vec2" | "Vec3" | "Vec4"
            | "Vec2i" | "Vec3i"
            | "AABB"
            | "Rect" | "Point" | "Color"
            | "Vec3Save"
    )
}

// =============================================================================
// CodeGenerator Type Inference Helpers
// =============================================================================
//
// These methods are used by CodeGenerator for expression type inference,
// method signature lookup, and usize detection. They are part of the split-impl
// pattern and can be called from any CodeGenerator impl block.

#[allow(clippy::collapsible_match, clippy::collapsible_if)]
impl<'ast> CodeGenerator<'ast> {
    /// BUG #8 FIX: Infer the type name from an expression
    /// This enables qualified method signature lookup (Type::method)
    pub(super) fn infer_type_name(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => {
                // "self" refers to the current struct type
                if name == "self" && self.in_impl_block {
                    return self.current_struct_name.clone();
                }
                // Try to infer from struct name if we're in an impl block
                if self.in_impl_block {
                    if let Some(struct_name) = &self.current_struct_name {
                        if self.current_struct_fields.contains(name) {
                            return Some(struct_name.clone());
                        }
                    }
                }
                // TDD FIX: Check function parameters for type info
                // e.g., fn test(validator: Validator) → infer_type_name("validator") = "Validator"
                for param in &self.current_function_params {
                    if param.name == *name {
                        return Self::type_to_name(&param.type_);
                    }
                }
                // TDD FIX: Check local variable types
                // e.g., let stack = Stack { .. } → infer_type_name("stack") = "Stack"
                if let Some(var_type) = self.local_var_types.get(name) {
                    return Self::type_to_name(var_type);
                }
                None
            }
            Expression::FieldAccess { object, field, .. } => {
                // TDD FIX: Try to resolve field type from struct field type tracking
                // e.g., self.transforms → World.transforms → ComponentArray<int> → "ComponentArray"
                let owner_type = self.infer_type_name(object);
                if let Some(ref owner) = owner_type {
                    // TDD FIX: For generic types like "ComponentArray<T>", also try base name "ComponentArray"
                    if let Some(field_types) =
                        self.struct_field_types.get(owner.as_str()).or_else(|| {
                            owner
                                .split('<')
                                .next()
                                .and_then(|base| self.struct_field_types.get(base))
                        })
                    {
                        if let Some(field_type) = field_types.get(field) {
                            if let Some(name) = Self::type_to_name(field_type) {
                                return Some(name);
                            }
                        }
                    }
                }
                // Fallback: use the owner type (for self.field_name → current struct type)
                owner_type
            }
            Expression::Unary {
                op:
                    crate::parser::UnaryOp::Deref
                    | crate::parser::UnaryOp::Ref
                    | crate::parser::UnaryOp::MutRef,
                operand,
                ..
            } => {
                // Look through references/derefs
                self.infer_type_name(operand)
            }
            Expression::MethodCall { object, .. } => {
                // Try to infer from the object
                self.infer_type_name(object)
            }
            Expression::Index { object, .. } => {
                // For array[i], the element type is unknown without full type inference
                // But we can try to infer the array type
                self.infer_type_name(object)
            }
            _ => None,
        }
    }

    /// Extract a type name from a Type enum (for signature lookup)
    pub(super) fn type_to_name(type_: &Type) -> Option<String> {
        match type_ {
            Type::Custom(name) => Some(name.clone()),
            Type::Parameterized(name, _) => Some(name.clone()),
            Type::Reference(inner) | Type::MutableReference(inner) => Self::type_to_name(inner),
            // TDD FIX: Handle stdlib container types for method signature lookup
            // Without this, self.dense (Vec<T>) can't resolve to "Vec" for Vec::remove lookup
            Type::Vec(_) => Some("Vec".to_string()),
            Type::Option(_) => Some("Option".to_string()),
            Type::Result(_, _) => Some("Result".to_string()),
            Type::Array(_, _) => Some("Array".to_string()),
            _ => None,
        }
    }

    /// Extract the element type from an iterable type.
    /// Vec<T> → T, &Vec<T> → T, &mut Vec<T> → T, Array(T, _) → T
    pub(super) fn extract_iterator_element_type(iterable_type: &Type) -> Option<Type> {
        match iterable_type {
            Type::Vec(inner) => Some(inner.as_ref().clone()),
            Type::Array(inner, _) => Some(inner.as_ref().clone()),
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::extract_iterator_element_type(inner)
            }
            _ => None,
        }
    }

    /// Infer the types of variables bound in match arm patterns.
    /// When matching `Some(x)` on `opt: Option<Stack>`, returns [("x", Type::Custom("Stack"))].
    /// This enables qualified method signature lookup for match-bound variables.
    pub(super) fn infer_match_bound_types(
        &self,
        scrutinee: &Expression,
        pattern: &Pattern,
    ) -> Vec<(String, Type)> {
        // Try to determine the scrutinee's type
        let scrutinee_type = self.infer_expression_type(scrutinee);
        let scrutinee_type = match scrutinee_type {
            Some(t) => t,
            None => return Vec::new(),
        };

        // Unwrap references: &T, &mut T → T
        let inner_type = match &scrutinee_type {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref().clone(),
            _ => scrutinee_type,
        };

        // Match pattern against type to extract bound variable types
        match (pattern, &inner_type) {
            // Some(var) matching Option<T> → var: T
            (
                Pattern::EnumVariant(variant, EnumPatternBinding::Single(var_name)),
                Type::Option(inner_t),
            ) if variant == "Some" => {
                vec![(var_name.clone(), inner_t.as_ref().clone())]
            }
            _ => Vec::new(),
        }
    }

    /// Try to infer the Type of an expression from local variable tracking and function parameters.
    pub(super) fn infer_expression_type(&self, expr: &Expression) -> Option<Type> {
        match expr {
            Expression::Identifier { name, .. } => {
                // Check local variable types first
                if let Some(t) = self.local_var_types.get(name) {
                    return Some(t.clone());
                }
                // Check function parameters
                for param in &self.current_function_params {
                    if param.name == *name {
                        return Some(param.type_.clone());
                    }
                }
                // In impl blocks, identifiers may refer to struct fields (implicit self)
                // e.g., `mouse_x` in `impl Game` → `self.mouse_x` → type is Game.mouse_x's type
                if self.in_impl_block && self.current_struct_fields.contains(name) {
                    if let Some(struct_name) = &self.current_struct_name {
                        if let Some(fields) = self.struct_field_types.get(struct_name.as_str()) {
                            if let Some(field_type) = fields.get(name.as_str()) {
                                return Some(field_type.clone());
                            }
                        }
                    }
                }
                None
            }
            // obj.field → look up field type from struct_field_types
            // Supports: self.field, var.field, and nested: self.config.max_size
            Expression::FieldAccess { object, field, .. } => {
                // Resolve the object's type first
                if let Expression::Identifier { name, .. } = &**object {
                    if name == "self" {
                        // self.field → current struct's field type
                        // TDD FIX: Also try base name for generic types
                        // e.g., "ComponentArray<T>" → try "ComponentArray"
                        if let Some(struct_name) = &self.current_struct_name {
                            if let Some(fields) = self
                                .struct_field_types
                                .get(struct_name.as_str())
                                .or_else(|| {
                                    struct_name
                                        .split('<')
                                        .next()
                                        .and_then(|base| self.struct_field_types.get(base))
                                })
                            {
                                if let Some(field_type) = fields.get(field.as_str()) {
                                    return Some(field_type.clone());
                                }
                            }
                        }
                    } else {
                        // var.field → look up var's type, then its field
                        // Check local variables first, then function parameters
                        let var_type =
                            self.local_var_types
                                .get(name.as_str())
                                .cloned()
                                .or_else(|| {
                                    self.current_function_params
                                        .iter()
                                        .find(|p| p.name == *name)
                                        .map(|p| p.type_.clone())
                                });
                        if let Some(var_type) = var_type {
                            let type_name = match &var_type {
                                Type::Custom(n) => n.as_str(),
                                // Handle references: &Recipe → Recipe, &mut Recipe → Recipe
                                Type::Reference(inner) | Type::MutableReference(inner) => {
                                    match inner.as_ref() {
                                        Type::Custom(n) => n.as_str(),
                                        _ => "",
                                    }
                                }
                                _ => "",
                            };
                            if let Some(fields) = self.struct_field_types.get(type_name) {
                                if let Some(field_type) = fields.get(field.as_str()) {
                                    return Some(field_type.clone());
                                }
                            }
                        }
                    }
                } else {
                    // Nested field access: self.config.max_size, obj.inner.field, etc.
                    // Recursively resolve the object's type, then look up the field
                    if let Some(obj_type) = self.infer_expression_type(object) {
                        let type_name = match &obj_type {
                            Type::Custom(n) => n.as_str(),
                            // Handle references: &Config → Config
                            Type::Reference(inner) | Type::MutableReference(inner) => {
                                match inner.as_ref() {
                                    Type::Custom(n) => n.as_str(),
                                    _ => "",
                                }
                            }
                            _ => "",
                        };
                        if !type_name.is_empty() {
                            // Also try stripping generic params: "Config<T>" → "Config"
                            let base_name = type_name.split('<').next().unwrap_or(type_name);
                            if let Some(fields) = self
                                .struct_field_types
                                .get(type_name)
                                .or_else(|| self.struct_field_types.get(base_name))
                            {
                                if let Some(field_type) = fields.get(field.as_str()) {
                                    return Some(field_type.clone());
                                }
                            }
                        }
                    }
                }
                None
            }
            // &expr or &mut expr → Reference(inner_type)
            Expression::Unary {
                op: crate::parser::UnaryOp::Ref,
                operand,
                ..
            } => self
                .infer_expression_type(operand)
                .map(|t| Type::Reference(Box::new(t))),
            Expression::Unary {
                op: crate::parser::UnaryOp::MutRef,
                operand,
                ..
            } => self
                .infer_expression_type(operand)
                .map(|t| Type::MutableReference(Box::new(t))),
            // Method calls: look up return type from method_return_types registry
            // and signature registry (for cross-file method resolution)
            Expression::MethodCall { object, method, .. } => {
                // Check well-known methods first
                if method == "len" || method == "count" || method == "capacity" {
                    return Some(Type::Custom("usize".to_string()));
                }
                // .clone() returns the same type as the object
                // This enables type inference through cloned iterables:
                //   for x in &collection.clone() → x has same element type as collection
                if method == "clone" {
                    return self.infer_expression_type(object);
                }
                // TDD FIX: .unwrap() on Option<T> → T
                if method == "unwrap" {
                    if let Some(obj_type) = self.infer_expression_type(object) {
                        if let Type::Option(inner) = obj_type {
                            return Some(*inner);
                        }
                    }
                }
                // Iterator methods: return the collection type so
                // extract_iterator_element_type can extract the element type.
                // This enables type inference for loop variables:
                //   for brick in self.bricks.iter_mut() → brick: Brick
                if method == "iter" || method == "iter_mut" || method == "into_iter" {
                    if let Some(obj_type) = self.infer_expression_type(object) {
                        return Some(obj_type);
                    }
                }
                // Look up from the method return type registry (populated during impl generation)
                if let Some(t) = self.method_return_types.get(method.as_str()) {
                    return Some(t.clone());
                }
                // TDD FIX: Cross-file method resolution via signature registry.
                // When the method is on a different type (e.g., animation.frame_count()),
                // method_return_types won't have it. Resolve the object's type, then
                // look up Type::method in the signature registry.
                if let Some(obj_type) = self.infer_expression_type(object) {
                    let type_name = match &obj_type {
                        Type::Custom(n) => n.clone(),
                        Type::Reference(inner) | Type::MutableReference(inner) => {
                            match inner.as_ref() {
                                Type::Custom(n) => n.clone(),
                                _ => String::new(),
                            }
                        }
                        _ => String::new(),
                    };
                    if !type_name.is_empty() {
                        let qualified = format!("{}::{}", type_name, method);
                        if let Some(sig) = self.signature_registry.get_signature(&qualified) {
                            return sig.return_type.clone();
                        }
                        // Also try base name for generic types
                        let base_name = type_name.split('<').next().unwrap_or(&type_name);
                        if base_name != type_name {
                            let qualified = format!("{}::{}", base_name, method);
                            if let Some(sig) = self.signature_registry.get_signature(&qualified) {
                                return sig.return_type.clone();
                            }
                        }
                    }
                }
                // Final fallback: try simple method name
                self.signature_registry
                    .get_signature(method)
                    .and_then(|sig| sig.return_type.clone())
            }
            // Block expression: infer from the last statement's expression
            // Handles: let x = { if cond { 64.0 } else { 32.0 } }
            Expression::Block { statements, .. } => {
                if let Some(last_stmt) = statements.last() {
                    match last_stmt {
                        Statement::Expression { expr, .. } => self.infer_expression_type(expr),
                        Statement::If { then_block, .. } => {
                            // Infer from the then branch's last expression
                            if let Some(last) = then_block.last() {
                                if let Statement::Expression { expr, .. } = last {
                                    return self.infer_expression_type(expr);
                                }
                            }
                            None
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            // Literal expressions: directly known types
            Expression::Literal { value, .. } => match value {
                Literal::Int(_) => Some(Type::Int),
                Literal::Float(_) => Some(Type::Float),
                Literal::Bool(_) => Some(Type::Bool),
                Literal::String(_) => Some(Type::String),
                _ => None,
            },
            // Binary operations: infer from operands (result usually matches operand type)
            Expression::Binary { left, right, .. } => self
                .infer_expression_type(left)
                .or_else(|| self.infer_expression_type(right)),
            // Cast expressions: the target type is explicit
            Expression::Cast { type_, .. } => Some(type_.clone()),
            // Call expressions: Type::method(args) → look up return type from signature registry
            // This is critical for Copy-type inference: let u = MathHelper::fade(x) → u is f32
            Expression::Call { function, .. } => {
                // Extract function name for signature lookup
                // Pattern: Type::method() → "Type::method"
                if let Expression::FieldAccess { object, field, .. } = function {
                    if let Expression::Identifier {
                        name: type_name, ..
                    } = object
                    {
                        let qualified = format!("{}::{}", type_name, field);
                        if let Some(sig) = self.signature_registry.get_signature(&qualified) {
                            return sig.return_type.clone();
                        }
                    }
                }
                // Pattern: simple function call → "function_name"
                if let Expression::Identifier { name, .. } = function {
                    if let Some(sig) = self.signature_registry.get_signature(name.as_str()) {
                        return sig.return_type.clone();
                    }
                }
                None
            }
            // TDD FIX: Index expressions: vec[i] → element type of the collection
            // Example: let mask: Vec<u8> = ...; let color_id = mask[i]; → color_id: u8
            Expression::Index { object, .. } => {
                if let Some(obj_type) = self.infer_expression_type(object) {
                    match obj_type {
                        Type::Vec(inner) => Some(*inner),
                        Type::Array(inner, _) => Some(*inner),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            // TDD FIX: Macro invocations return known types
            // format!() always returns String
            // vec![] returns Vec<T> (but we don't infer T here)
            Expression::MacroInvocation { name, .. } => {
                match name.as_str() {
                    "format" => Some(Type::String),
                    "panic" => None, // Never returns (diverges)
                    "println" | "print" | "eprintln" | "eprint" => None, // Returns ()
                    "vec" => None, // TODO: Could infer Vec<T> from element types
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Check if an expression produces usize (e.g., .len(), array indexing)
    /// Used for auto-casting between i32 and usize in comparisons
    pub(crate) fn expression_produces_usize(&self, expr: &Expression) -> bool {
        match expr {
            // .len() returns usize
            Expression::MethodCall { method, .. } => {
                if method == "len" || method == "count" || method == "capacity" {
                    return true;
                }
                // Fallback: check via type inference
                self.infer_expression_type_is_usize(expr)
            }
            // Binary ops with usize operands: i + 1, len() - 1, etc.
            // TDD FIX (Bug #4): If BOTH sides are usize (or one side is usize and other is int literal),
            // then the result is usize. The old logic used OR which was wrong.
            Expression::Binary {
                op,
                left,
                right,
                location: _,
            } => {
                match op {
                    // Arithmetic operations preserve usize if both operands are usize-compatible
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod => {
                        let left_is_usize = self.expression_produces_usize(left);
                        let right_is_usize = self.expression_produces_usize(right);

                        // Int literals adapt to the other operand's type
                        let right_is_literal = matches!(**right, Expression::Literal { .. });
                        let left_is_literal = matches!(**left, Expression::Literal { .. });

                        // Result is usize if:
                        // - Both are usize, OR
                        // - One is usize and the other is an int literal
                        (left_is_usize && (right_is_usize || right_is_literal))
                            || (right_is_usize && left_is_literal)
                    }
                    // Comparison/logical operations don't produce usize
                    _ => false,
                }
            }
            // Casts to usize: (x as usize)
            Expression::Cast { type_, .. } => {
                matches!(type_, Type::Custom(name) if name == "usize")
            }
            // Variables assigned from .len() or typed as usize
            Expression::Identifier { name, .. } => {
                if self.usize_variables.contains(name) {
                    return true;
                }

                // Check if this is a struct field with usize type (in impl block)
                if self.in_impl_block && self.current_struct_fields.contains(name) {
                    // Look up the struct to see if this field is usize
                    // Strip generic parameters: "Pool<T>" → "Pool"
                    if let Some(struct_name) = &self.current_struct_name {
                        let base_name = struct_name.split('<').next().unwrap_or(struct_name);
                        if let Some(usize_fields) = self.usize_struct_fields.get(base_name) {
                            if usize_fields.contains(name) {
                                return true;
                            }
                        }
                    }
                }

                // Fallback: check parameters and local variable types via type inference
                self.infer_expression_type_is_usize(expr)
            }
            // Field access: self.field_name or obj.field_name (including nested)
            Expression::FieldAccess { object, field, .. } => {
                // Check if accessing a usize field on self (fast path)
                if let Expression::Identifier { name: obj_name, .. } = &**object {
                    if obj_name == "self" && self.in_impl_block {
                        // Look up struct to see if this field is usize
                        if let Some(struct_name) = &self.current_struct_name {
                            // Strip generic parameters: "Pool<T>" → "Pool"
                            let base_name = struct_name.split('<').next().unwrap_or(struct_name);
                            if let Some(usize_fields) = self.usize_struct_fields.get(base_name) {
                                if usize_fields.contains(field) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                // Fallback: use type inference for obj.field, self.config.field, etc.
                self.infer_expression_type_is_usize(expr)
            }
            _ => false,
        }
    }

    /// Check if an expression's inferred type is usize.
    /// Uses infer_expression_type() for comprehensive type resolution including
    /// parameters, local variables, nested field access, and method return types.
    pub(super) fn infer_expression_type_is_usize(&self, expr: &Expression) -> bool {
        if let Some(t) = self.infer_expression_type(expr) {
            return matches!(t, Type::Custom(ref name) if name == "usize");
        }
        false
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
