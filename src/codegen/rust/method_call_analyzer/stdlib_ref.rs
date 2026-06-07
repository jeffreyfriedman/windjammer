//! Stdlib method heuristics for automatic `&` insertion when no analyzer signature exists.

use crate::parser::{Expression, OwnershipHint, Type};

use super::{MethodCallAnalyzer, MethodCallContext};

impl MethodCallAnalyzer {
    /// Check if this method call needs & based on stdlib patterns
    pub(super) fn needs_stdlib_ref(
        method: &str,
        arg: &Expression,
        ctx: &MethodCallContext<'_, '_>,
        arg_count: usize,
        receiver_type_name: Option<&str>,
        local_var_types: Option<&std::collections::HashMap<String, Type>>,
    ) -> bool {
        let usize_variables = ctx.usize_variables;
        let current_function_params = ctx.current_function_params;
        let borrowed_iterator_vars = ctx.borrowed_iterator_vars;
        let inferred_borrowed_params = ctx.inferred_borrowed_params;

        let arg_is_already_borrowed = if let Expression::Identifier { name, .. } = arg {
            let is_ref_param = current_function_params.iter().any(|p| {
                &p.name == name && matches!(p.ownership, OwnershipHint::Ref | OwnershipHint::Mut)
            });
            let is_borrowed_iter_var = borrowed_iterator_vars.contains(name);
            let param_is_ref = current_function_params.iter().any(|p| {
                p.name == *name
                    && crate::codegen::rust::types::param_generates_as_rust_ref(
                        &p.type_, &p.name, inferred_borrowed_params,
                    )
            });

            is_ref_param || is_borrowed_iter_var || param_is_ref
        } else {
            false
        };

        if arg_is_already_borrowed {
            return false;
        }

        if arg_count > 1 && super::super::stdlib_method_traits::is_map_key_method(method) {
            return false;
        }

        if super::super::stdlib_method_traits::is_map_key_method(method) {
            let is_known_map = receiver_type_name.is_some_and(|n| {
                let base = n.split('<').next().unwrap_or(n);
                matches!(base, "HashMap" | "BTreeMap" | "IndexMap")
            });
            let is_known_vec =
                receiver_type_name.is_some_and(|n| n.split('<').next().unwrap_or(n) == "Vec");

            if is_known_vec {
                return false;
            }
            if is_known_map {
                if let Expression::Identifier { name, .. } = arg {
                    let is_already_ref = current_function_params.iter().any(|p| {
                        p.name == *name
                            && crate::codegen::rust::types::param_generates_as_rust_ref(
                                &p.type_, &p.name, inferred_borrowed_params,
                            )
                    });
                    if is_already_ref {
                        return false;
                    }
                }
                return true;
            }

            if super::super::stdlib_method_traits::is_map_key_method(method)
                && Self::is_copy_type_with_locals(arg, usize_variables, current_function_params, local_var_types)
            {
                let arg_name = if let Expression::Identifier { name, .. } = arg {
                    Some(name.as_str())
                } else {
                    None
                };

                let looks_like_hashmap_key = arg_name.is_some_and(|name| {
                    name == "id"
                        || name == "key"
                        || name == "entity"
                        || name.ends_with("_id")
                        || name.ends_with("_key")
                });

                if looks_like_hashmap_key {
                    if let Some(name) = arg_name {
                        let is_already_map_key_ref = current_function_params.iter().any(|p| {
                            p.name == name
                                && crate::codegen::rust::types::param_generates_as_rust_ref(
                                    &p.type_, &p.name, inferred_borrowed_params,
                                )
                        });
                        if is_already_map_key_ref {
                            return false;
                        }
                    }
                    return true;
                }

                return false;
            }

            if super::super::stdlib_method_traits::is_map_key_method(method) {
                if let Expression::Cast { type_, .. } = arg {
                    if Self::is_copy_type_annotation_internal(type_) {
                        return false;
                    }
                }
            }

            return true;
        }

        if Self::is_copy_type_with_locals(arg, usize_variables, current_function_params, local_var_types) {
            return false;
        }

        if matches!(method, "contains" | "binary_search") {
            return true;
        }

        if matches!(method, "contains" | "starts_with" | "ends_with") {
            return true;
        }

        false
    }
}
