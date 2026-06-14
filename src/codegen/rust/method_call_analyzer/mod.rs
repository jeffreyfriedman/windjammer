#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]

//! Method Call Analyzer — automatic conversions for method arguments (`&`, `.clone()`, `.to_string()`, …).

mod analyze_borrow_signature;
mod clone_string;
mod copy_detection;
mod registry_lookup;
mod stdlib_ref;

use crate::analyzer::OwnershipMode;
use crate::parser::{Expression, Literal, OwnershipHint, Parameter, Type};
use std::collections::HashSet;

/// Context for method call analysis, grouping related parameter collections
pub struct MethodCallContext<'a, 'ast> {
    pub usize_variables: &'a HashSet<String>,
    pub current_function_params: &'a [Parameter<'ast>],
    pub borrowed_iterator_vars: &'a HashSet<String>,
    pub inferred_borrowed_params: &'a HashSet<String>,
    pub str_ref_optimized_params: &'a HashSet<String>,
}

/// Analyzes method calls to determine what automatic conversions are needed
pub struct MethodCallAnalyzer;

/// Whether a named argument already generates as a Rust reference at the call site.
fn arg_identifier_already_generates_as_rust_ref(
    name: &str,
    current_function_params: &[Parameter],
    inferred_borrowed_params: &HashSet<String>,
    str_ref_optimized_params: &HashSet<String>,
    borrowed_iterator_vars: &HashSet<String>,
) -> bool {
    if borrowed_iterator_vars.contains(name) || str_ref_optimized_params.contains(name) {
        return true;
    }
    current_function_params.iter().any(|param| {
        param.name == name
            && (matches!(param.ownership, OwnershipHint::Ref | OwnershipHint::Mut)
                || crate::codegen::rust::types::param_generates_as_rust_ref(
                    &param.type_,
                    &param.name,
                    inferred_borrowed_params,
                ))
    })
}

impl MethodCallAnalyzer {
    /// True when codegen spells the callee's formal parameter `sig_param_idx` as `&str` in Rust
    pub fn callee_param_is_rust_str_slice(
        method_signature: &Option<crate::analyzer::FunctionSignature>,
        sig_param_idx: usize,
    ) -> bool {
        method_signature
            .as_ref()
            .and_then(|sig| sig.param_types.get(sig_param_idx))
            .is_some_and(|pt| {
                matches!(
                    pt,
                    Type::Reference(inner)
                        if crate::codegen::rust::types::is_windjammer_text_type(inner)
                            || matches!(&**inner, Type::Custom(s) if s == "str")
                )
            })
    }

    /// Determine if we should add & to this argument
    #[allow(clippy::too_many_arguments)]
    pub fn should_add_ref(
        arg: &Expression,
        arg_str: &str,
        method: &str,
        param_idx: usize,
        method_signature: &Option<crate::analyzer::FunctionSignature>,
        usize_variables: &HashSet<String>,
        current_function_params: &[Parameter],
        borrowed_iterator_vars: &HashSet<String>,
        inferred_borrowed_params: &HashSet<String>,
        arg_count: usize,
        receiver_type_name: Option<&str>,
        local_var_types: Option<&std::collections::HashMap<String, Type>>,
        stdlib_signatures: Option<
            &std::collections::HashMap<
                String,
                std::collections::HashMap<String, crate::codegen::rust::generator::MethodSignature>,
            >,
        >,
        user_signatures: Option<
            &std::collections::HashMap<
                String,
                std::collections::HashMap<String, crate::codegen::rust::generator::MethodSignature>,
            >,
        >,
        match_arm_bindings: &HashSet<String>,
        str_ref_optimized_params: &HashSet<String>,
    ) -> bool {
        let is_string_literal = matches!(
            arg,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        );
        if is_string_literal {
            return false;
        }

        let is_integer_literal = matches!(
            arg,
            Expression::Literal {
                value: Literal::Int(_) | Literal::IntSuffixed(_, _),
                ..
            }
        );
        if is_integer_literal {
            return false;
        }

        let is_float_literal = matches!(
            arg,
            Expression::Literal {
                value: Literal::Float(_),
                ..
            }
        );
        if is_float_literal {
            return false;
        }

        let is_bool_literal = matches!(
            arg,
            Expression::Literal {
                value: Literal::Bool(_),
                ..
            }
        );
        if is_bool_literal {
            return false;
        }

        if matches!(arg, Expression::StructLiteral { .. }) {
            return false;
        }

        if arg_str.starts_with('&') {
            return false;
        }

        if matches!(
            arg,
            Expression::Unary {
                op: crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef,
                ..
            }
        ) {
            return false;
        }

        if let Expression::Identifier { name, .. } = arg {
            if match_arm_bindings.contains(name.as_str()) {
                if let Some(var_types) = local_var_types {
                    if let Some(ty) = var_types.get(name.as_str()) {
                        if matches!(ty, Type::Reference(_) | Type::MutableReference(_)) {
                            return false;
                        }
                    }
                }

                if let Some(sig) = method_signature {
                    let sig_param_idx = if sig.has_self_receiver {
                        param_idx + 1
                    } else {
                        param_idx
                    };
                    if let Some(&OwnershipMode::Borrowed) = sig.param_ownership.get(sig_param_idx) {
                        return true;
                    }
                    if let Some(&ownership) = sig.param_ownership.get(sig_param_idx) {
                        if matches!(ownership, OwnershipMode::Owned | OwnershipMode::MutBorrowed) {
                            return false;
                        }
                    }
                }

                if let Some(var_types) = local_var_types {
                    if let Some(ty) = var_types.get(name.as_str()) {
                        let is_string = matches!(ty, Type::String)
                            || matches!(ty, Type::Custom(s) if s == "String" || s == "string");
                        if is_string {
                            return true;
                        }
                    }
                }
            }
        }

        // Must run before registry_lookup: stdlib signatures say HashMap.get wants `&K`, but
        // `key: str` / `key: &string` already spell as `&str` in Rust — adding `&` → &&str.
        if let Expression::Identifier { name, .. } = arg {
            if arg_identifier_already_generates_as_rust_ref(
                name,
                current_function_params,
                inferred_borrowed_params,
                str_ref_optimized_params,
                borrowed_iterator_vars,
            ) {
                return false;
            }
        }

        if let Some(receiver_type) = receiver_type_name {
            if let Some(decision) = registry_lookup::ref_from_signature_registries(
                receiver_type,
                method,
                param_idx,
                arg,
                stdlib_signatures,
                user_signatures,
                local_var_types,
                current_function_params,
                borrowed_iterator_vars,
                inferred_borrowed_params,
                str_ref_optimized_params,
            ) {
                return decision;
            }
        }

        if matches!(arg, Expression::MethodCall { .. }) {
            let is_map_key_method = matches!(
                method,
                "get" | "get_mut" | "contains_key" | "remove" | "get_key_value"
            ) && param_idx == 0;
            let is_known_map = receiver_type_name.is_some_and(|n| {
                let base = n.split('<').next().unwrap_or(n);
                crate::type_classification::is_map_type(base)
            });
            if is_map_key_method && is_known_map {
                return true;
            }
            if let Some(sig) = method_signature {
                let sig_param_idx = if sig.has_self_receiver {
                    param_idx + 1
                } else {
                    param_idx
                };
                let is_borrowed = sig
                    .param_ownership
                    .get(sig_param_idx)
                    .is_some_and(|&o| matches!(o, OwnershipMode::Borrowed));
                if !is_borrowed {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Expression::Identifier { name, .. } = arg {
            if borrowed_iterator_vars.contains(name) {
                return false;
            }
        }

        let is_hashmap_key_method = matches!(
            method,
            "get" | "get_mut" | "contains_key" | "remove" | "get_key_value"
        ) && param_idx == 0
            && receiver_type_name.is_some_and(|n| {
                let base = n.split('<').next().unwrap_or(n);
                crate::type_classification::is_map_type(base)
            });

        if is_hashmap_key_method {
            if let Expression::Identifier { name, .. } = arg {
                if arg_identifier_already_generates_as_rust_ref(
                    name,
                    current_function_params,
                    inferred_borrowed_params,
                    str_ref_optimized_params,
                    borrowed_iterator_vars,
                ) {
                    return false;
                }
            }
        }

        if matches!(
            arg,
            Expression::Unary {
                op: crate::parser::UnaryOp::Deref,
                ..
            }
        ) {
            return false;
        }

        if let Expression::Cast { type_, .. } = arg {
            if Self::is_copy_type_annotation_internal(type_) {
                let is_known_map = receiver_type_name.is_some_and(|n| {
                    let base = n.split('<').next().unwrap_or(n);
                    matches!(base, "HashMap" | "BTreeMap" | "IndexMap")
                });
                let is_map_key_method = matches!(
                    method,
                    "get" | "get_mut" | "remove" | "contains_key" | "get_key_value"
                ) && param_idx == 0;
                if !(is_known_map && is_map_key_method) {
                    return false;
                }
            }
        }

        if let Some(sig) = method_signature {
            if let Some(decision) = analyze_borrow_signature::ref_from_known_method_signature(
                sig,
                param_idx,
                arg,
                current_function_params,
                inferred_borrowed_params,
                local_var_types,
            ) {
                return decision;
            }
        }

        let ctx = MethodCallContext {
            usize_variables,
            current_function_params,
            borrowed_iterator_vars,
            inferred_borrowed_params,
            str_ref_optimized_params,
        };
        Self::needs_stdlib_ref(
            method,
            arg,
            &ctx,
            arg_count,
            receiver_type_name,
            local_var_types,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_type_detection() {
        let usize_vars = HashSet::new();
        let params = vec![];

        let expr = Expression::Identifier {
            name: "sparse_idx_usize".to_string(),
            location: Default::default(),
        };
        assert!(
            !MethodCallAnalyzer::is_copy_type(&expr, &usize_vars, &params),
            "should NOT assume variable is Copy based on name alone"
        );

        let expr = Expression::Identifier {
            name: "entity".to_string(),
            location: Default::default(),
        };
        assert!(
            !MethodCallAnalyzer::is_copy_type(&expr, &usize_vars, &params),
            "should NOT assume entity is Copy"
        );

        let mut usize_vars_with = HashSet::new();
        usize_vars_with.insert("my_index".to_string());
        let expr = Expression::Identifier {
            name: "my_index".to_string(),
            location: Default::default(),
        };
        assert!(
            MethodCallAnalyzer::is_copy_type(&expr, &usize_vars_with, &params),
            "registered usize variables should be Copy"
        );

        let params_with_type = vec![crate::parser::Parameter {
            name: "count".to_string(),
            pattern: None,
            type_: crate::parser::Type::Custom("i32".to_string()),
            ownership: crate::parser::ast::ownership::OwnershipHint::Inferred,
            is_mutable: false,
            decorators: vec![],
        }];
        let expr = Expression::Identifier {
            name: "count".to_string(),
            location: Default::default(),
        };
        assert!(
            MethodCallAnalyzer::is_copy_type(&expr, &usize_vars, &params_with_type),
            "typed i32 parameters should be Copy"
        );
    }
}
