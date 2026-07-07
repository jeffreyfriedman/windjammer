//! `.clone()` and `.to_string()` decisions for method call arguments.

use crate::analyzer::OwnershipMode;
use crate::parser::{Expression, Parameter, Type};
use std::collections::HashSet;

use super::MethodCallAnalyzer;

impl MethodCallAnalyzer {
    /// Phase-2 lookup-key APIs: first `string` param becomes `&str` in Rust even when
    /// stale metadata still lists `String` + `Owned` (Blackboard, SaveData getters, etc.).
    pub fn is_lookup_key_string_param(method: &str, param_idx: usize) -> bool {
        crate::codegen::rust::string_utilities::is_readonly_string_key_method(method, param_idx)
    }

    /// Determine if we should add .clone() to this argument
    #[allow(clippy::too_many_arguments)]
    pub fn should_add_clone(
        arg: &Expression,
        arg_str: &str,
        method: &str,
        param_idx: usize,
        method_signature: &Option<crate::analyzer::FunctionSignature>,
        borrowed_iterator_vars: &HashSet<String>,
        current_function_params: &[Parameter],
        inferred_borrowed_params: &HashSet<String>,
        current_function_return_type: &Option<Type>,
    ) -> bool {
        if matches!(arg, Expression::MethodCall { .. }) {
            return false;
        }

        if let Expression::Identifier { name, .. } = arg {
            if borrowed_iterator_vars.contains(name) && !arg_str.ends_with(".clone()") {
                if method == "push" {
                    if let Some(Type::Vec(inner_type)) = current_function_return_type {
                        if matches!(**inner_type, Type::Reference(_) | Type::MutableReference(_)) {
                            return false;
                        }
                    }

                    return true;
                }

                if let Some(sig) = method_signature {
                    if let Some(&ownership) = sig.param_ownership_for_arg(param_idx) {
                        if matches!(ownership, OwnershipMode::Owned) {
                            return true;
                        }
                    }
                }
            }
        }

        if let Some(sig) = method_signature {
            if let Some(&ownership) = sig.param_ownership_for_arg(param_idx) {
                if matches!(ownership, OwnershipMode::Borrowed) {
                    return false;
                }

                if matches!(ownership, OwnershipMode::Owned) {
                    if let Expression::FieldAccess { object, .. } = arg {
                        if let Expression::Identifier { name, .. } = &**object {
                            let is_explicitly_borrowed = current_function_params.iter().any(|p| {
                                &p.name == name
                                    && matches!(p.ownership, crate::parser::OwnershipHint::Ref)
                            });
                            let is_inferred_borrowed = inferred_borrowed_params.contains(name);
                            if (is_explicitly_borrowed || is_inferred_borrowed)
                                && !arg_str.ends_with(".clone()")
                            {
                                if !Self::is_copy_type(
                                    arg,
                                    &HashSet::new(),
                                    current_function_params,
                                ) {
                                    let param_is_copy =
                                        sig.param_type_for_arg(param_idx).is_some_and(|t| {
                                            crate::codegen::rust::type_analysis::is_copy_type(t)
                                        });
                                    if !param_is_copy {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// Determine if we should add .to_string() to this string literal
    pub fn should_add_to_string(
        param_idx: usize,
        method: &str,
        method_signature: &Option<crate::analyzer::FunctionSignature>,
    ) -> bool {
        crate::codegen::rust::string_utilities::string_literal_needs_owned_coercion(
            method_signature.as_ref(),
            param_idx,
            Some(method),
        )
    }

    /// Determine if we should add .cloned() for Option<&T> -> Option<T>
    pub fn should_add_cloned(method: &str, _return_type: &Option<Type>) -> bool {
        matches!(
            method,
            "get" | "get_mut" | "contains_key" | "remove" | "get_key_value"
        ) || matches!(method, "unwrap" | "first" | "last")
    }
}
