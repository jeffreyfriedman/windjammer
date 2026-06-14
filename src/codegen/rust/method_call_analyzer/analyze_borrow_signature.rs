//! Analyzer-owned function signature paths for reference insertion (`Borrowed` parameters).

use crate::analyzer::{FunctionSignature, OwnershipMode};
use crate::parser::{Expression, Parameter, Type};
use std::collections::HashMap;
use std::collections::HashSet;

use super::MethodCallAnalyzer;

#[allow(clippy::too_many_arguments)]
pub(super) fn ref_from_known_method_signature(
    sig: &FunctionSignature,
    param_idx: usize,
    arg: &Expression,
    current_function_params: &[Parameter],
    inferred_borrowed_params: &HashSet<String>,
    local_var_types: Option<&HashMap<String, Type>>,
) -> Option<bool> {
    let sig_param_idx = if sig.has_self_receiver {
        param_idx + 1
    } else {
        param_idx
    };
    let ownership = sig.param_ownership.get(sig_param_idx)?;
    if !matches!(ownership, OwnershipMode::Borrowed) {
        return Some(false);
    }

    // `if let Some(x) = &self.opt` binds `x` as `&T`. Do not add another `&` for `&T` params.
    if let Expression::Identifier { name: arg_name, .. } = arg {
        if let Some(var_types) = local_var_types {
            if let Some(ty) = var_types.get(arg_name) {
                if matches!(ty, Type::Reference(_) | Type::MutableReference(_)) {
                    return Some(false);
                }
            }
        }
    }

    if let Some(param_type) = sig.param_types.get(sig_param_idx) {
        if !matches!(param_type, Type::Reference(_) | Type::MutableReference(_))
            && MethodCallAnalyzer::is_copy_type_annotation_internal(param_type)
        {
            return Some(false);
        }
    }

    if let Some(param_type) = sig.param_types.get(sig_param_idx) {
        let param_is_str_ref = match param_type {
            Type::Reference(inner) => matches!(&**inner, Type::Custom(s) if s == "str"),
            _ => false,
        };

        if param_is_str_ref {
            if let Expression::Identifier { name: arg_name, .. } = arg {
                let already_rust_str = current_function_params.iter().any(|p| {
                    p.name == *arg_name
                        && crate::codegen::rust::types::param_generates_as_rust_ref(
                            &p.type_,
                            &p.name,
                            inferred_borrowed_params,
                        )
                });
                if already_rust_str {
                    return Some(false);
                }

                let is_owned_string_param = current_function_params.iter().any(|p| {
                    p.name == *arg_name
                        && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                        && !inferred_borrowed_params.contains(arg_name)
                });

                let is_owned_string_local = local_var_types
                    .as_ref()
                    .and_then(|vars| vars.get(arg_name))
                    .is_some_and(crate::codegen::rust::types::is_windjammer_text_type);

                if is_owned_string_param || is_owned_string_local {
                    return Some(true);
                }
            }
        }
    }

    if let Expression::Identifier { name: arg_name, .. } = arg {
        let already_rust_str = current_function_params.iter().any(|p| {
            p.name == *arg_name
                && crate::codegen::rust::types::param_generates_as_rust_ref(
                    &p.type_,
                    &p.name,
                    inferred_borrowed_params,
                )
        });
        if already_rust_str {
            return Some(false);
        }
    }
    Some(true)
}
