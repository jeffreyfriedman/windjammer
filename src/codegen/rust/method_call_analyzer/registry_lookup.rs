//! Type-registry–based hints for whether a method argument needs `&` (stdlib + user signatures).

use crate::parser::{Expression, Parameter, Type};
use std::collections::HashMap;
use std::collections::HashSet;

use crate::codegen::rust::generator::MethodSignature;

type StdlibSigMap<'a> = &'a HashMap<String, HashMap<String, MethodSignature>>;
type UserSigMap<'a> = &'a HashMap<String, HashMap<String, MethodSignature>>;

/// Returns `Some(true)` when registry lookup proves the argument must be referenced.
#[allow(clippy::too_many_arguments)]
pub(super) fn ref_from_signature_registries(
    receiver_type: &str,
    method: &str,
    param_idx: usize,
    arg: &Expression,
    stdlib_signatures: Option<StdlibSigMap<'_>>,
    user_signatures: Option<UserSigMap<'_>>,
    local_var_types: Option<&HashMap<String, Type>>,
    current_function_params: &[Parameter],
    borrowed_iterator_vars: &HashSet<String>,
    inferred_borrowed_params: &HashSet<String>,
) -> Option<bool> {
    if let Some(stdlib_sigs) = stdlib_signatures {
        let base_type = receiver_type.split('<').next().unwrap_or(receiver_type);

        if let Some(methods) = stdlib_sigs.get(base_type) {
            if let Some(sig) = methods.get(method) {
                if let Some(param_type) = sig.param_types.get(param_idx) {
                    let param_is_str_ref = matches!(
                        param_type,
                        Type::Reference(inner) if matches!(&**inner, Type::Custom(s) if s == "str")
                    );

                    if param_is_str_ref {
                        if let Expression::Identifier { name, .. } = arg {
                            if let Some(local_types) = local_var_types {
                                if let Some(var_type) = local_types.get(name.as_str()) {
                                    if crate::codegen::rust::types::is_windjammer_text_type(
                                        var_type,
                                    ) {
                                        return Some(true);
                                    }
                                }
                            }

                            let is_owned_string = current_function_params.iter().any(|p| {
                                p.name == *name
                                    && crate::codegen::rust::types::is_windjammer_text_type(
                                        &p.type_,
                                    )
                                    && !inferred_borrowed_params.contains(name.as_str())
                            });
                            if is_owned_string {
                                return Some(true);
                            }
                        }
                    }

                    if let Some(&ownership) = sig.param_ownership.get(param_idx) {
                        if matches!(ownership, crate::analyzer::OwnershipMode::Borrowed) {
                            if let Expression::Identifier { name, .. } = arg {
                                if inferred_borrowed_params.contains(name.as_str()) {
                                    return Some(false);
                                }
                                if borrowed_iterator_vars.contains(name.as_str()) {
                                    return Some(false);
                                }
                            }
                            return Some(true);
                        }
                    }
                }
            }
        }
    }

    if let Some(user_sigs) = user_signatures {
        if let Some(methods) = user_sigs.get(receiver_type) {
            if let Some(sig) = methods.get(method) {
                if let Some(param_type) = sig.param_types.get(param_idx) {
                    let param_is_str_ref = matches!(
                        param_type,
                        Type::Reference(inner) if matches!(&**inner, Type::Custom(s) if s == "str")
                    );

                    if param_is_str_ref {
                        if let Expression::Identifier { name, .. } = arg {
                            if let Some(local_types) = local_var_types {
                                if let Some(var_type) = local_types.get(name.as_str()) {
                                    if crate::codegen::rust::types::is_windjammer_text_type(
                                        var_type,
                                    ) {
                                        return Some(true);
                                    }
                                }
                            }

                            let is_owned_string = current_function_params.iter().any(|p| {
                                p.name == *name
                                    && crate::codegen::rust::types::is_windjammer_text_type(
                                        &p.type_,
                                    )
                                    && !inferred_borrowed_params.contains(name.as_str())
                            });
                            if is_owned_string {
                                return Some(true);
                            }
                        }
                    }

                    if let Some(&ownership) = sig.param_ownership.get(param_idx) {
                        if matches!(ownership, crate::analyzer::OwnershipMode::Borrowed) {
                            if let Expression::Identifier { name, .. } = arg {
                                if inferred_borrowed_params.contains(name.as_str()) {
                                    return Some(false);
                                }
                                if borrowed_iterator_vars.contains(name.as_str()) {
                                    return Some(false);
                                }
                            }
                            return Some(true);
                        }
                    }
                }
            }
        }
    }

    None
}
