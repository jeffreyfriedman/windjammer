//! Rust integer inherent / `Ord` method signatures for [`SignatureRegistry`].
//!
//! `i32::max` / `min` / `clamp` / `abs` return the receiver width. Without these
//! keys, `let max_size = w.max(h).max(d)` stays untyped and reuse analysis emits
//! `max_size.clone()` (WDB-343). No method-name ownership lists — registry return
//! type is the source of truth.

use crate::parser::Type;

use super::{FunctionSignature, OwnershipMode, SignatureRegistry};

struct MethodDef {
    name: &'static str,
    extra_args: usize,
    signed_only: bool,
}

const INHERENT_INT_METHODS: &[MethodDef] = &[
    MethodDef {
        name: "max",
        extra_args: 1,
        signed_only: false,
    },
    MethodDef {
        name: "min",
        extra_args: 1,
        signed_only: false,
    },
    MethodDef {
        name: "clamp",
        extra_args: 2,
        signed_only: false,
    },
    MethodDef {
        name: "abs",
        extra_args: 0,
        signed_only: true,
    },
];

const SIGNED: &[&str] = &["i8", "i16", "i32", "i64", "i128", "isize"];
const UNSIGNED: &[&str] = &["u8", "u16", "u32", "u64", "u128", "usize"];

fn build_int_method_sig(int_name: &str, def: &MethodDef) -> FunctionSignature {
    let int_ty = Type::Custom(int_name.to_string());
    let mut param_types = vec![int_ty.clone()];
    // Copy primitives: Rust `Ord::max(self, other: Self)` is by value.
    let mut param_ownership = vec![OwnershipMode::Owned];
    for _ in 0..def.extra_args {
        param_types.push(int_ty.clone());
        param_ownership.push(OwnershipMode::Owned);
    }
    FunctionSignature {
        name: format!("{int_name}::{}", def.name),
        param_types: param_types.clone(),
        formal_param_types: param_types,
        param_ownership,
        return_type: Some(int_ty),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: true,
        is_extern: false,
        emitted_rust_ref_params: None,
        string_ref_string_formal_params: None,
        field_extract_params: None,
        forwarding_borrow_params: None,
    }
}

/// Register `i32::max` / `usize::min` / … inherent signatures on `registry`.
pub fn register_primitive_int_signatures(registry: &mut SignatureRegistry) {
    for int_name in SIGNED.iter().chain(UNSIGNED.iter()) {
        for def in INHERENT_INT_METHODS {
            if def.signed_only && UNSIGNED.contains(int_name) {
                continue;
            }
            let sig = build_int_method_sig(int_name, def);
            registry.add_function(sig.name.clone(), sig);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::stdlib_method_traits::lookup_method_signature;

    #[test]
    fn i32_max_is_registered_owned_self() {
        let reg = SignatureRegistry::stdlib();
        let sig = lookup_method_signature("max", Some("i32"), &reg)
            .expect("i32::max boundary signature");
        assert_eq!(sig.name, "i32::max");
        assert_eq!(sig.return_type.as_ref(), Some(&Type::Custom("i32".into())));
        assert_eq!(sig.param_ownership[0], OwnershipMode::Owned);
        assert_eq!(sig.param_ownership[1], OwnershipMode::Owned);
    }

    #[test]
    fn usize_min_is_registered() {
        let reg = SignatureRegistry::stdlib();
        let sig = lookup_method_signature("min", Some("usize"), &reg)
            .expect("usize::min boundary signature");
        assert_eq!(sig.return_type.as_ref(), Some(&Type::Custom("usize".into())));
    }

    #[test]
    fn unsigned_has_no_abs() {
        let reg = SignatureRegistry::stdlib();
        assert!(lookup_method_signature("abs", Some("u32"), &reg).is_none());
        assert!(lookup_method_signature("abs", Some("i32"), &reg).is_some());
    }
}
