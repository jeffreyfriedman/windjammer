//! Trait Derivation Module
//!
//! Handles automatic trait derivation for structs and enums.
//! Determines which traits (Debug, Clone, Copy, PartialEq, Eq, Hash, Default)
//! can be safely derived based on field/variant types.

use crate::codegen::rust::type_analysis;
use crate::parser::*;

use super::CodeGenerator;

impl CodeGenerator<'_> {
    pub(super) fn infer_derivable_traits(&self, struct_: &StructDecl) -> Vec<String> {
        let has_trait_object_field = struct_
            .fields
            .iter()
            .any(|f| self.type_contains_trait_object(&f.field_type));

        let mut traits = if has_trait_object_field {
            vec![]
        } else {
            vec!["Debug".to_string(), "Clone".to_string()]
        };

        if !has_trait_object_field && self.all_fields_are_copy(&struct_.fields) {
            traits.push("Copy".to_string());
        }

        if self.all_fields_are_partial_eq(&struct_.fields) {
            traits.push("PartialEq".to_string());

            if self.all_fields_are_eq(&struct_.fields) {
                traits.push("Eq".to_string());

                if self.all_fields_are_hashable(&struct_.fields) {
                    traits.push("Hash".to_string());
                }
            }
        }

        if self.all_fields_have_default(&struct_.fields) {
            traits.push("Default".to_string());
        }

        traits
    }

    /// Check if a type contains a trait object (dyn Trait) anywhere in its structure.
    /// Used to prevent auto-deriving Debug/Clone on structs containing Box<dyn Trait>.
    pub(super) fn type_contains_trait_object(&self, type_: &Type) -> bool {
        match type_ {
            Type::TraitObject(_) => true,
            Type::Vec(inner)
            | Type::Option(inner)
            | Type::Reference(inner)
            | Type::MutableReference(inner) => self.type_contains_trait_object(inner),
            Type::Parameterized(_, args) => args.iter().any(|a| self.type_contains_trait_object(a)),
            Type::Result(ok, err) => {
                self.type_contains_trait_object(ok) || self.type_contains_trait_object(err)
            }
            Type::Array(inner, _) => self.type_contains_trait_object(inner),
            Type::Tuple(types) => types.iter().any(|t| self.type_contains_trait_object(t)),
            _ => false,
        }
    }

    fn all_fields_are_copy(&self, fields: &[crate::parser::StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.is_copy_type_with_registry(&field.field_type))
    }

    /// TDD FIX: Check if a type is Copy, including user-defined types with @derive(Copy)
    pub(super) fn is_copy_type_with_registry(&self, ty: &Type) -> bool {
        use crate::parser::Type;
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
            Type::Reference(_) => true,
            Type::MutableReference(_) => false,
            Type::RawPointer { .. } => true,
            Type::Tuple(types) => types.iter().all(|t| self.is_copy_type_with_registry(t)),
            Type::Custom(name) => {
                let is_primitive = matches!(
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
                );
                is_primitive || self.copy_types_registry.contains(name.as_str())
            }
            _ => false,
        }
    }

    fn all_fields_are_partial_eq(&self, fields: &[crate::parser::StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.is_partial_eq_type(&field.field_type))
    }

    /// Check if all enum variants have only Copy fields.
    pub(super) fn all_enum_variants_are_copy(
        &self,
        variants: &[crate::parser::EnumVariant],
    ) -> bool {
        use crate::parser::EnumVariantData;
        variants.iter().all(|variant| match &variant.data {
            EnumVariantData::Unit => true,
            EnumVariantData::Tuple(types) => types.iter().all(type_analysis::is_copy_type),
            EnumVariantData::Struct(fields) => fields
                .iter()
                .all(|(_, field_type)| type_analysis::is_copy_type(field_type)),
        })
    }

    pub(super) fn all_enum_variants_are_partial_eq(
        &self,
        variants: &[crate::parser::EnumVariant],
    ) -> bool {
        use crate::parser::EnumVariantData;
        variants.iter().all(|variant| match &variant.data {
            EnumVariantData::Unit => true,
            EnumVariantData::Tuple(types) => types.iter().all(|ty| self.is_partial_eq_type(ty)),
            EnumVariantData::Struct(fields) => fields
                .iter()
                .all(|(_, field_type)| self.is_partial_eq_type(field_type)),
        })
    }

    /// Pre-pass: Collect which custom types (structs/enums) support PartialEq
    pub(super) fn collect_partial_eq_types(&mut self, program: &Program) {
        for item in &program.items {
            match item {
                Item::Struct { decl: s, .. } => {
                    let has_auto = s.decorators.iter().any(|d| d.name == "auto");
                    if has_auto {
                        let all_fields_support_partial_eq = s
                            .fields
                            .iter()
                            .all(|f| self.is_partial_eq_type_recursive(&f.field_type));
                        if all_fields_support_partial_eq {
                            self.partial_eq_types.insert(s.name.clone());
                        }
                    }
                }
                Item::Enum { decl: e, .. } => {
                    if self.all_enum_variants_are_partial_eq_recursive(&e.variants) {
                        self.partial_eq_types.insert(e.name.clone());
                    }
                    self.collect_enum_variant_types(e);
                }
                _ => {}
            }
        }
    }

    /// Collect field types for each enum variant into the enum_variant_types registry.
    fn collect_enum_variant_types(&mut self, e: &crate::parser::EnumDecl) {
        use crate::parser::EnumVariantData;
        for variant in &e.variants {
            let key = format!("{}::{}", e.name, variant.name);
            match &variant.data {
                EnumVariantData::Unit => {
                    self.enum_variant_types.insert(key, vec![]);
                }
                EnumVariantData::Tuple(types) => {
                    self.enum_variant_types.insert(key, types.clone());
                }
                EnumVariantData::Struct(fields) => {
                    let types: Vec<Type> = fields.iter().map(|(_, ty)| ty.clone()).collect();
                    self.enum_variant_types.insert(key, types);
                }
            }
        }
    }

    fn is_partial_eq_type_recursive(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool | Type::String => true,
            Type::Custom(name)
                if matches!(
                    name.as_str(),
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
                        | "char"
                ) =>
            {
                true
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.is_partial_eq_type_recursive(inner)
            }
            Type::Tuple(types) => types.iter().all(|t| self.is_partial_eq_type_recursive(t)),
            Type::Vec(inner) => self.is_partial_eq_type_recursive(inner),
            Type::Option(inner) => self.is_partial_eq_type_recursive(inner),
            Type::Result(ok, err) => {
                self.is_partial_eq_type_recursive(ok) && self.is_partial_eq_type_recursive(err)
            }
            _ => false,
        }
    }

    fn all_enum_variants_are_partial_eq_recursive(
        &self,
        variants: &[crate::parser::EnumVariant],
    ) -> bool {
        use crate::parser::EnumVariantData;
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

    fn all_fields_are_eq(&self, fields: &[crate::parser::StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.is_eq_type(&field.field_type))
    }

    fn all_fields_are_hashable(&self, fields: &[crate::parser::StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.is_hashable_type(&field.field_type))
    }

    fn all_fields_have_default(&self, fields: &[crate::parser::StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.has_default(&field.field_type))
    }

    #[allow(clippy::only_used_in_recursion)]
    #[allow(clippy::only_used_in_recursion)]
    fn is_partial_eq_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool | Type::String => true,
            Type::Custom(name)
                if matches!(
                    name.as_str(),
                    "String"
                        | "f32"
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
                        | "char"
                ) =>
            {
                true
            }
            Type::Custom(name) => self.partial_eq_types.contains(name),
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.is_partial_eq_type(inner)
            }
            Type::Tuple(types) => types.iter().all(|t| self.is_partial_eq_type(t)),
            Type::Vec(inner) => self.is_partial_eq_type(inner),
            Type::Option(inner) => self.is_partial_eq_type(inner),
            Type::Result(ok, err) => self.is_partial_eq_type(ok) && self.is_partial_eq_type(err),
            _ => false,
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn is_eq_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Bool | Type::String => true,
            Type::Float => false,
            Type::Custom(name) if matches!(name.as_str(), "f32" | "f64") => false,
            Type::Custom(name)
                if matches!(
                    name.as_str(),
                    "String"
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
                        | "char"
                ) =>
            {
                true
            }
            Type::Reference(inner) | Type::MutableReference(inner) => self.is_eq_type(inner),
            Type::Tuple(types) => types.iter().all(|t| self.is_eq_type(t)),
            Type::Vec(inner) => self.is_eq_type(inner),
            Type::Option(inner) => self.is_eq_type(inner),
            Type::Result(ok, err) => self.is_eq_type(ok) && self.is_eq_type(err),
            _ => false,
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    pub(super) fn is_hashable_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Bool | Type::String => true,
            Type::Float => false,
            Type::Custom(name) if matches!(name.as_str(), "f32" | "f64") => false,
            Type::Custom(name)
                if matches!(
                    name.as_str(),
                    "String"
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
                        | "char"
                ) =>
            {
                true
            }
            Type::Reference(inner) => self.is_hashable_type(inner),
            Type::MutableReference(_) => false,
            Type::Tuple(types) => types.iter().all(|t| self.is_hashable_type(t)),
            Type::Vec(_) => false,
            Type::Option(inner) => self.is_hashable_type(inner),
            _ => false,
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    pub(super) fn has_default(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
            Type::String => true,
            Type::Vec(_) => true,
            Type::Option(_) => true,
            Type::Tuple(types) => types.iter().all(|t| self.has_default(t)),
            Type::Custom(name)
                if matches!(
                    name.as_str(),
                    "String"
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
                        | "f32"
                        | "f64"
                        | "bool"
                        | "char"
                ) =>
            {
                true
            }
            _ => false,
        }
    }
}
