//! Rust type string emission (`type_to_rust`), Copy detection, and formatted generic parameters.

use crate::codegen::rust::generator::CodeGenerator;
use crate::parser::Type;

impl<'ast> CodeGenerator<'ast> {
    pub(crate) fn type_to_rust(&self, type_: &Type) -> String {
        // When the user has import aliases (e.g., `use std::collections::HashMap as Map`),
        // skip stdlib type mappings for those alias names so the alias is preserved in output.
        let aliases = &self.import_aliases;
        let map = &self.extern_submodule_qualifiers;
        if map.is_empty() && aliases.is_empty() {
            return crate::codegen::rust::types::type_to_rust(type_);
        }
        let qualify = move |s: &str| {
            let dotted = s.replace('.', "::");
            if !map.is_empty() {
                crate::codegen::rust::codegen_helpers::qualify_parent_child_external_path(
                    map, &dotted,
                )
            } else {
                dotted
            }
        };
        if aliases.is_empty() {
            crate::codegen::rust::types::type_to_rust_mapped(type_, &qualify)
        } else {
            crate::codegen::rust::types::type_to_rust_mapped_with_aliases(type_, &qualify, aliases)
        }
    }

    /// Check if a type implements Copy.
    ///
    /// Handles:
    /// 1. Primitives (via type_analysis::is_copy_type)
    /// 2. Option<T> when T is Copy (Option<f32>, Option<AABB>, etc.)
    /// 3. User structs with @derive(Copy) (copy_types_registry)
    /// 4. Structs with all-Copy fields (struct_field_types recursive check)
    /// 5. Known game engine types from external crates (Vec3, AABB, etc.)
    pub(super) fn is_type_copy(&self, ty: &Type) -> bool {
        if crate::codegen::rust::type_analysis::is_copy_type(ty) {
            return true;
        }
        match ty {
            Type::Option(inner) => self.is_type_copy(inner),
            Type::Custom(name) => {
                if self.copy_types_registry.contains(name.as_str()) {
                    return true;
                }
                crate::codegen::rust::type_analysis::is_known_copy_type(name.as_str())
            }
            _ => false,
        }
    }

    // Example: [TypeParam { name: "T", bounds: ["Display", "Clone"] }] -> "T: Display + Clone"
    pub(crate) fn format_type_params(&self, type_params: &[crate::parser::TypeParam]) -> String {
        type_params
            .iter()
            .map(|param| {
                if param.bounds.is_empty() {
                    param.name.clone()
                } else {
                    // Expand bound aliases
                    let expanded_bounds: Vec<String> = param
                        .bounds
                        .iter()
                        .flat_map(|bound| {
                            // Check if this bound is an alias
                            if let Some(traits) = self.bound_aliases.get(bound) {
                                traits.clone()
                            } else {
                                vec![bound.clone()]
                            }
                        })
                        .collect();
                    format!("{}: {}", param.name, expanded_bounds.join(" + "))
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}
