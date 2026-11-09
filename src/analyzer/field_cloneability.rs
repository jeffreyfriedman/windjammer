//! Whether a field type can be copied with `.clone()`. A method that moves a field out of `self` borrows `self`
//! and clones at the read (C5) only when the field has a `Clone`; otherwise the method must consume `self`.

use crate::parser::Type;

use super::Analyzer;

impl<'ast> Analyzer<'ast> {
    /// The type has no `Clone` in the emitted Rust: a std type the derive rules leave without it, a trait object, or a
    /// program struct holding one (checked through the struct's fields, a few levels deep).
    pub(super) fn field_type_cannot_be_cloned(&self, ty: &Type, depth: u8) -> bool {
        use crate::type_classification::{is_shared_ownership_wrapper, is_std_non_auto_debug_clone_type};
        match ty {
            Type::TraitObject(_) | Type::ImplTrait(_) => true,
            Type::Vec(inner) | Type::Option(inner) => self.field_type_cannot_be_cloned(inner, depth + 1),
            Type::Parameterized(base, args) => {
                if is_shared_ownership_wrapper(base) {
                    false
                } else if is_std_non_auto_debug_clone_type(base) {
                    true
                } else {
                    args.iter().any(|a| self.field_type_cannot_be_cloned(a, depth + 1)) || self.struct_has_uncloneable_field(base, depth)
                }
            }
            Type::Custom(name) => is_std_non_auto_debug_clone_type(name) || self.struct_has_uncloneable_field(name, depth),
            _ => false,
        }
    }

    fn struct_has_uncloneable_field(&self, name: &str, depth: u8) -> bool {
        if depth >= 4 {
            return false;
        }
        let leaf = name.rsplit("::").next().unwrap_or(name);
        if let Some(ctx) = self.self_impl_context.as_ref() {
            for item in &ctx.program().items {
                if let crate::parser::Item::Struct { decl, .. } = item {
                    if decl.name == leaf {
                        return decl.fields.iter().any(|f| self.field_type_cannot_be_cloned(&f.field_type, depth + 1));
                    }
                }
            }
        }
        self.global_struct_field_types.get(leaf).is_some_and(|fields| fields.values().any(|t| self.field_type_cannot_be_cloned(t, depth + 1)))
    }
}
