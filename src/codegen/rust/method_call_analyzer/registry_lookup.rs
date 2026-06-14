//! Type-registry–based hints for whether a method argument needs `&` (stdlib + user signatures).

use crate::parser::{Expression, OwnershipHint, Parameter, Type};
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
    _user_signatures: Option<UserSigMap<'_>>,
    local_var_types: Option<&HashMap<String, Type>>,
    current_function_params: &[Parameter],
    borrowed_iterator_vars: &HashSet<String>,
    inferred_borrowed_params: &HashSet<String>,
    str_ref_optimized_params: &HashSet<String>,
) -> Option<bool> {
    let stdlib_sigs = stdlib_signatures?;
    let base_type = receiver_type.split('<').next().unwrap_or(receiver_type);
    let methods = stdlib_sigs.get(base_type)?;
    let sig = methods.get(method)?;
    let param_type = sig.param_types.get(param_idx)?;

    let param_is_str_ref = matches!(
        param_type,
        Type::Reference(inner) if matches!(&**inner, Type::Custom(s) if s == "str")
    );

    if param_is_str_ref {
        if let Some(result) = check_str_ref_arg(
            arg,
            current_function_params,
            inferred_borrowed_params,
            local_var_types,
        ) {
            return Some(result);
        }
    }

    if let Some(&ownership) = sig.param_ownership.get(param_idx) {
        if matches!(ownership, crate::analyzer::OwnershipMode::Borrowed) {
            if let Expression::Identifier { name, .. } = arg {
                if arg_already_ref(
                    name,
                    current_function_params,
                    inferred_borrowed_params,
                    borrowed_iterator_vars,
                    str_ref_optimized_params,
                ) {
                    return Some(false);
                }
            }
            return Some(true);
        }
    }

    None
}

/// When a signature parameter is `&str`, decide whether the argument already is
/// a reference (→ `Some(false)`), needs `&` added (→ `Some(true)`), or is
/// indeterminate (→ `None`).
fn check_str_ref_arg(
    arg: &Expression,
    current_function_params: &[Parameter],
    inferred_borrowed_params: &HashSet<String>,
    local_var_types: Option<&HashMap<String, Type>>,
) -> Option<bool> {
    let Expression::Identifier { name, .. } = arg else {
        return None;
    };

    if current_function_params.iter().any(|p| {
        p.name == *name
            && crate::codegen::rust::types::param_generates_as_rust_ref(
                &p.type_,
                &p.name,
                inferred_borrowed_params,
            )
    }) {
        return Some(false);
    }

    if let Some(local_types) = local_var_types {
        if let Some(var_type) = local_types.get(name.as_str()) {
            if crate::codegen::rust::types::is_windjammer_text_type(var_type) {
                return Some(true);
            }
        }
    }

    let is_owned_string = current_function_params.iter().any(|p| {
        p.name == *name
            && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
            && !inferred_borrowed_params.contains(name.as_str())
    });
    if is_owned_string {
        return Some(true);
    }

    None
}

/// Whether the named argument is already a reference and should NOT get `&`.
fn arg_already_ref(
    name: &str,
    current_function_params: &[Parameter],
    inferred_borrowed_params: &HashSet<String>,
    borrowed_iterator_vars: &HashSet<String>,
    str_ref_optimized_params: &HashSet<String>,
) -> bool {
    if str_ref_optimized_params.contains(name) {
        return true;
    }
    if current_function_params.iter().any(|p| {
        p.name == name
            && (matches!(p.ownership, OwnershipHint::Ref | OwnershipHint::Mut)
                || crate::codegen::rust::types::param_generates_as_rust_ref(
                    &p.type_,
                    &p.name,
                    inferred_borrowed_params,
                ))
    }) {
        return true;
    }
    if borrowed_iterator_vars.contains(name) {
        return true;
    }
    false
}
