//! Stdlib method heuristics for automatic `&` insertion when no analyzer signature exists.

use crate::parser::{Expression, OwnershipHint, Type};

use super::{MethodCallAnalyzer, MethodCallContext};

impl MethodCallAnalyzer {
    /// Check if this method call needs & based on stdlib patterns
    pub(super) fn needs_stdlib_ref(
        method: &str,
        arg: &Expression,
        ctx: &MethodCallContext<'_, '_>,
        _arg_count: usize,
        receiver_type_name: Option<&str>,
        local_var_types: Option<&std::collections::HashMap<String, Type>>,
        signature_registry: Option<&crate::analyzer::SignatureRegistry>,
        param_idx: usize,
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
                        &p.type_,
                        &p.name,
                        inferred_borrowed_params,
                    )
            });
            let is_str_ref_optimized = ctx.str_ref_optimized_params.contains(name.as_str());

            is_ref_param || is_borrowed_iter_var || param_is_ref || is_str_ref_optimized
        } else {
            false
        };

        if arg_is_already_borrowed {
            return false;
        }

        // Signature-driven collection key / borrowed-element args — no method-name lists.
        if let Some(registry) = signature_registry {
            if super::super::stdlib_method_traits::method_arg_expects_borrowed_reference_qualified(
                method,
                receiver_type_name,
                registry,
                param_idx,
            ) {
                if let Expression::Identifier { name, .. } = arg {
                    let is_already_ref = current_function_params.iter().any(|p| {
                        p.name == *name
                            && crate::codegen::rust::types::param_generates_as_rust_ref(
                                &p.type_,
                                &p.name,
                                inferred_borrowed_params,
                            )
                    });
                    if is_already_ref {
                        return false;
                    }
                }

                if let Expression::Cast { type_, .. } = arg {
                    if Self::is_copy_type_annotation_internal(type_) {
                        return false;
                    }
                }

                if Self::is_copy_type_with_locals(
                    arg,
                    usize_variables,
                    current_function_params,
                    local_var_types,
                ) {
                    return true;
                }

                return true;
            }
        }

        if Self::is_copy_type_with_locals(
            arg,
            usize_variables,
            current_function_params,
            local_var_types,
        ) {
            return false;
        }

        // Signature-driven `&str` Pattern slots and slice-search borrows — no method-name lists.
        if let Some(registry) = signature_registry {
            let recv = receiver_type_name.or(Some("String"));
            if super::super::stdlib_method_traits::method_arg_expects_rust_str_ref_qualified(
                method,
                recv,
                registry,
                param_idx,
            ) {
                return true;
            }
            if crate::analyzer::stdlib_method_traits::method_is_slice_search_qualified(
                method,
                receiver_type_name,
                registry,
            ) {
                return true;
            }
        }

        false
    }
}
