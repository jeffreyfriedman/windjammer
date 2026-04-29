//! Trait Derivation Module
//!
//! Handles automatic trait derivation for structs and enums.
//! Determines which traits (Debug, Clone, Copy, PartialEq, Eq, Hash, Default)
//! can be safely derived based on field/variant types.

use crate::codegen::rust::type_analysis;
use crate::parser::*;

use super::CodeGenerator;

impl CodeGenerator<'_> {
    /// Merge user-listed `#[derive]` traits with compiler-inferred ones.
    ///
    /// Partial `@derive(Clone)` (or `@auto(Clone)`) must not disable Windjammer auto-derive:
    /// nested types and enums still need `Debug` (and other safe traits) for `println!("{:?}")`,
    /// `#[derive(Debug)]` on parent enums, etc.
    ///
    /// Standard library derivable traits are emitted in stable order; any other traits
    /// (e.g. `Serialize`, `Parser`) follow in sorted order.
    pub(super) fn merge_standard_derive_traits(
        explicit: Vec<String>,
        inferred: Vec<String>,
    ) -> Vec<String> {
        use std::collections::HashSet;
        const STANDARD_ORDER: &[&str] = &[
            "Debug",
            "Clone",
            "Copy",
            "PartialEq",
            "Eq",
            "PartialOrd",
            "Ord",
            "Hash",
            "Default",
        ];
        let mut names: HashSet<String> = explicit.iter().cloned().collect();
        names.extend(inferred);
        let mut out = Vec::new();
        for &t in STANDARD_ORDER {
            if names.remove(t) {
                out.push(t.to_string());
            }
        }
        let mut extra: Vec<String> = names.into_iter().collect();
        extra.sort();
        out.extend(extra);
        out
    }

    pub(super) fn infer_derivable_traits(&self, struct_: &StructDecl) -> Vec<String> {
        // For tuple structs, check tuple_fields instead of named fields
        let all_types: Vec<&Type> = if let Some(ref tuple_fields) = struct_.tuple_fields {
            tuple_fields.iter().collect()
        } else {
            struct_.fields.iter().map(|f| &f.field_type).collect()
        };

        let has_trait_object_field = all_types.iter().any(|t| self.type_contains_trait_object(t));

        let mut traits = if has_trait_object_field {
            vec![]
        } else {
            vec!["Debug".to_string(), "Clone".to_string()]
        };

        let all_copy = all_types.iter().all(|t| self.is_copy_type_with_registry(t));
        if !has_trait_object_field && all_copy {
            traits.push("Copy".to_string());
        }

        if self.all_fields_are_partial_eq(&struct_.fields) {
            // For tuple structs, check tuple field types directly
            let partial_eq_ok = if struct_.tuple_fields.is_some() {
                all_types.iter().all(|t| self.is_partial_eq_type(t))
            } else {
                true
            };
            if partial_eq_ok {
                traits.push("PartialEq".to_string());

                let eq_ok = if struct_.tuple_fields.is_some() {
                    all_types.iter().all(|t| self.is_eq_type(t))
                } else {
                    self.all_fields_are_eq(&struct_.fields)
                };
                if eq_ok {
                    traits.push("Eq".to_string());

                    let hash_ok = if struct_.tuple_fields.is_some() {
                        all_types.iter().all(|t| self.is_hashable_type(t))
                    } else {
                        self.all_fields_are_hashable(&struct_.fields)
                    };
                    if hash_ok {
                        traits.push("Hash".to_string());
                    }
                }
            }
        }

        let default_ok = if struct_.tuple_fields.is_some() {
            all_types.iter().all(|t| self.has_default(t))
        } else {
            self.all_fields_have_default(&struct_.fields)
        };
        if default_ok {
            traits.push("Default".to_string());
        }

        traits
    }

    /// Check if a type contains a trait object (dyn Trait) anywhere in its structure.
    /// Used to prevent auto-deriving Debug/Clone on structs containing Box<dyn Trait>.
    /// Also checks for ImplTrait because `trait X` in struct fields becomes Box<dyn X>.
    ///
    /// User-defined struct names use `trait_object_types` (filled by `collect_trait_object_types`)
    /// so an outer struct is treated as containing a trait object when a field's type is a struct
    /// that (transitively) holds `dyn` / `trait X`.
    pub(super) fn type_contains_trait_object(&self, type_: &Type) -> bool {
        match type_ {
            Type::TraitObject(_) | Type::ImplTrait(_) => true,
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
                false
            }
            Type::Custom(name) => self.trait_object_types.contains(name),
            _ => false,
        }
    }

    /// Fixpoint pass: record every struct in this program that transitively contains a trait object.
    pub(super) fn collect_trait_object_types(&mut self, program: &Program) {
        loop {
            let sz = self.trait_object_types.len();

            for item in &program.items {
                if let Item::Struct { decl: s, .. } = item {
                    if self.trait_object_types.contains(&s.name) {
                        continue;
                    }
                    if s.fields
                        .iter()
                        .any(|f| self.type_contains_trait_object(&f.field_type))
                    {
                        self.trait_object_types.insert(s.name.clone());
                    }
                }
            }

            if self.trait_object_types.len() == sz {
                break;
            }
        }
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
    ///
    /// Codegen auto-derives `PartialEq` for structs without `@derive` / `@auto` when all fields
    /// support it, and for `@auto` with inferred traits — but nested custom types were only
    /// registered when the inner struct had `@auto`, and recursive checks ignored the registry.
    /// That broke parent structs and enums comparing fields of child structs. Fix: fixpoint
    /// registration mirroring `struct_emits_partial_eq` + `is_partial_eq_type`, and interleave
    /// structs with enums so mutually dependent types converge.
    pub(super) fn collect_partial_eq_types(&mut self, program: &Program) {
        for item in &program.items {
            if let Item::Enum { decl: e, .. } = item {
                self.collect_enum_variant_types(e);
            }
        }

        loop {
            let sz = self.partial_eq_types.len();

            for item in &program.items {
                if let Item::Struct { decl: s, .. } = item {
                    if self.partial_eq_types.contains(&s.name) {
                        continue;
                    }
                    if !self.struct_emits_partial_eq(s) {
                        continue;
                    }
                    if s.fields
                        .iter()
                        .all(|f| self.is_partial_eq_type(&f.field_type))
                    {
                        self.partial_eq_types.insert(s.name.clone());
                    }
                }
            }

            for item in &program.items {
                if let Item::Enum { decl: e, .. } = item {
                    if self.partial_eq_types.contains(&e.name) {
                        continue;
                    }
                    if self.all_enum_variants_are_partial_eq(&e.variants) {
                        self.partial_eq_types.insert(e.name.clone());
                    }
                }
            }

            if self.partial_eq_types.len() == sz {
                break;
            }
        }
    }

    /// True if `generate_struct` will emit a `#[derive(...)]` that includes `PartialEq`
    /// (explicit `@derive` / `@auto`, or implicit auto-derive when there is no derive decorator).
    fn struct_emits_partial_eq(&self, s: &StructDecl) -> bool {
        let has_auto_or_derive = s
            .decorators
            .iter()
            .any(|d| d.name == "auto" || d.name == "derive");

        let mut saw_partial_eq = false;
        for d in &s.decorators {
            if d.name == "derive" {
                if Self::decorator_identifier_traits(d)
                    .iter()
                    .any(|t| t == "PartialEq")
                {
                    saw_partial_eq = true;
                }
            } else if d.name == "auto" {
                let traits = if d.arguments.is_empty() {
                    self.infer_derivable_traits(s)
                } else {
                    Self::decorator_identifier_traits(d)
                };
                if traits.iter().any(|t| t == "PartialEq") {
                    saw_partial_eq = true;
                }
            }
        }

        if saw_partial_eq {
            return true;
        }

        if !has_auto_or_derive {
            return self
                .infer_derivable_traits(s)
                .iter()
                .any(|t| t == "PartialEq");
        }

        false
    }

    fn decorator_identifier_traits(d: &crate::parser::Decorator<'_>) -> Vec<String> {
        d.arguments
            .iter()
            .filter_map(|(_key, expr)| {
                if let Expression::Identifier { name, .. } = expr {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect()
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
                    self.enum_variant_types.insert(key.clone(), types);
                    self.enum_variant_struct_fields.insert(key, fields.clone());
                }
            }
        }
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
            Type::Array(inner, _) => self.is_partial_eq_type(inner),
            Type::Parameterized(base, args) => {
                matches!(
                    base.as_str(),
                    "Vec"
                        | "Option"
                        | "Result"
                        | "Box"
                        | "Rc"
                        | "Arc"
                        | "HashMap"
                        | "BTreeMap"
                        | "Cow"
                ) && args.iter().all(|a| self.is_partial_eq_type(a))
            }
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
