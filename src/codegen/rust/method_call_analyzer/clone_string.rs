//! `.clone()` and `.to_string()` decisions for method call arguments.

use crate::analyzer::OwnershipMode;
use crate::parser::{Expression, Parameter, Type};
use std::collections::HashSet;

use super::MethodCallAnalyzer;

impl MethodCallAnalyzer {
    /// Phase-2 lookup-key APIs: first `string` param becomes `&str` in Rust even when
    /// stale metadata still lists `String` + `Owned` (Blackboard, SaveData getters, etc.).
    pub fn is_lookup_key_string_param(
        method: &str,
        param_idx: usize,
        sig: Option<&crate::analyzer::FunctionSignature>,
        receiver_type: Option<&str>,
        registry: Option<&crate::analyzer::SignatureRegistry>,
    ) -> bool {
        crate::codegen::rust::string_utilities::is_readonly_string_key_method(
            method, param_idx, sig, receiver_type, registry,
        )
    }

    /// Determine if we should add .clone() to this argument
    #[allow(clippy::too_many_arguments)]
    pub fn should_add_clone(
        arg: &Expression,
        arg_str: &str,
        _method: &str,
        param_idx: usize,
        method_signature: &Option<crate::analyzer::FunctionSignature>,
        borrowed_iterator_vars: &HashSet<String>,
        current_function_params: &[Parameter],
        inferred_borrowed_params: &HashSet<String>,
        current_function_return_type: &Option<crate::parser::Type>,
    ) -> bool {
        if let Some(sig) = method_signature {
            if crate::codegen::rust::stdlib_method_traits::callee_arg_expects_reference_param(
                sig, param_idx,
            ) {
                return false;
            }
            if sig
                .param_type_for_arg(param_idx)
                .is_some_and(crate::codegen::rust::type_analysis::is_copy_type)
            {
                return false;
            }
        }

        if matches!(arg, Expression::MethodCall { .. }) {
            return false;
        }

        if let Expression::Identifier { name, .. } = arg {
            if borrowed_iterator_vars.contains(name) {
                if matches!(
                    current_function_return_type,
                    Some(Type::Vec(inner)) if matches!(**inner, Type::Reference(_) | Type::MutableReference(_))
                ) {
                    return false;
                }
                if !arg_str.ends_with(".clone()") {
                    if let Some(sig) = method_signature {
                        if crate::codegen::rust::stdlib_method_traits::callee_arg_expects_reference_param(
                            sig, param_idx,
                        ) {
                            return false;
                        }
                        if let Some(param_type) = sig.param_type_for_arg(param_idx) {
                            if crate::codegen::rust::type_analysis::is_copy_type(param_type) {
                                return false;
                            }
                        }
                        if let Some(&ownership) = sig.param_ownership_for_arg(param_idx) {
                            if matches!(ownership, OwnershipMode::Owned) {
                                return true;
                            }
                        }
                    }
                    // No signature → do not guess from method names (`push`/`insert`/…).
                    // Call sites must resolve stdlib/user signatures via the registry.
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

    /// Determine if we should add .cloned() for Option<&T> -> Option<T>.
    /// Driven by the resolved return type (or signature return), never method names.
    pub fn should_add_cloned(
        _method: &str,
        return_type: &Option<crate::parser::Type>,
        method_signature: Option<&crate::analyzer::FunctionSignature>,
    ) -> bool {
        let ret = method_signature
            .and_then(|s| s.return_type.as_ref())
            .or(return_type.as_ref());
        match ret {
            Some(Type::Parameterized(base, args))
                if matches!(base.as_str(), "Option" | "option") && args.len() == 1 =>
            {
                matches!(
                    args[0],
                    Type::Reference(_) | Type::MutableReference(_)
                )
            }
            _ => false,
        }
    }
}
