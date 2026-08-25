//! IR-neutral formal/type predicates for call-site emission contracts.
//!
//! Pure functions over `FunctionSignature` and `Type` — no codegen state.

use crate::analyzer::FunctionSignature;
use crate::parser::Type;

/// Windjammer text types (`string`, `String`, `str`, and references thereto).
pub fn is_windjammer_text_type(t: &Type) -> bool {
    matches!(t, Type::String)
        || matches!(
            t,
            Type::Custom(name) if matches!(name.as_str(), "string" | "String" | "str")
        )
        || matches!(t, Type::Reference(inner) if is_windjammer_text_type(inner))
}

/// True when `ty` is Windjammer/Rust owned string (`string` / `String`).
pub fn type_is_owned_string(ty: &Type) -> bool {
    matches!(ty, Type::String) || matches!(ty, Type::Custom(n) if n == "String" || n == "string")
}

/// Parameter type is Rust shared `&str`.
pub fn param_is_rust_str_ref(param_type: &Type) -> bool {
    matches!(
        param_type,
        Type::Reference(inner) if matches!(**inner, Type::Custom(ref n) if n == "str")
    )
}

/// Parameter type is Rust shared `&String`.
pub fn param_is_rust_string_ref(param_type: &Type) -> bool {
    matches!(
        param_type,
        Type::Reference(inner) if type_is_owned_string(inner)
    )
}

/// Bare WJ `string` formal (not source-level `&T` / `&mut T`).
pub fn formal_is_plain_windjammer_string(sig: &FunctionSignature, param_idx: usize) -> bool {
    sig.formal_param_type(param_idx).is_some_and(|t| {
        !matches!(t, Type::Reference(_) | Type::MutableReference(_))
            && is_windjammer_text_type(t)
    })
}

/// Like [`formal_is_plain_windjammer_string`] but for a call-site argument index (accounts for `self`).
pub fn formal_is_plain_windjammer_string_for_call_arg(
    sig: &FunctionSignature,
    arg_index: usize,
) -> bool {
    let pidx = sig.arg_param_index(arg_index);
    if formal_is_plain_windjammer_string(sig, pidx) {
        return true;
    }
    sig.has_self_receiver
        && sig.formal_param_types.get(arg_index).is_some_and(|t| {
            !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                && is_windjammer_text_type(t)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::{FunctionSignature, OwnershipMode};
    use crate::parser::Type;

    fn sig_with_formal_types(types: Vec<Type>) -> FunctionSignature {
        FunctionSignature {
            name: "test_fn".into(),
            formal_param_types: types.clone(),
            param_types: types,
            param_ownership: vec![OwnershipMode::Owned],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        }
    }

    #[test]
    fn plain_windjammer_string_formal_without_ref_wrapper() {
        let sig = sig_with_formal_types(vec![Type::String]);
        assert!(formal_is_plain_windjammer_string(&sig, 0));
    }

    #[test]
    fn explicit_ref_string_formal_is_not_plain() {
        let sig = sig_with_formal_types(vec![Type::Reference(Box::new(Type::Custom("str".into())))]);
        assert!(!formal_is_plain_windjammer_string(&sig, 0));
    }

    #[test]
    fn param_is_rust_str_ref_detects_converged_str() {
        assert!(param_is_rust_str_ref(&Type::Reference(Box::new(Type::Custom(
            "str".into()
        )))));
        assert!(!param_is_rust_str_ref(&Type::String));
    }

    #[test]
    fn method_call_arg_index_accounts_for_self_layout() {
        let sig = FunctionSignature {
            name: "M::f".into(),
            formal_param_types: vec![Type::String],
            param_types: vec![Type::Custom("M".into()), Type::String],
            param_ownership: vec![OwnershipMode::Owned, OwnershipMode::Owned],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        assert!(formal_is_plain_windjammer_string_for_call_arg(&sig, 1));
    }
}
