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

/// Rust `to_*_bytes` / `from_*_bytes` width. `usize`/`isize` follow this compiler's
/// 64-bit Rust target (`[u8; 8]`).
fn primitive_endian_width(name: &str) -> Option<usize> {
    match name {
        "i8" | "u8" => Some(1),
        "i16" | "u16" => Some(2),
        "i32" | "u32" | "f32" => Some(4),
        "i64" | "u64" | "f64" => Some(8),
        "i128" | "u128" => Some(16),
        "isize" | "usize" => Some(8),
        _ => None,
    }
}

fn byte_array(width: usize) -> Type {
    Type::Array(Box::new(Type::Custom("u8".into())), width)
}

fn endian_self_to_bytes(prim: &str, method: &str, width: usize) -> FunctionSignature {
    let self_ty = Type::Custom(prim.to_string());
    FunctionSignature {
        name: format!("{prim}::{method}"),
        param_types: vec![self_ty.clone()],
        formal_param_types: vec![self_ty],
        // Copy primitive: Rust `to_le_bytes(self)` is by value.
        param_ownership: vec![OwnershipMode::Owned],
        return_type: Some(byte_array(width)),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: true,
        is_extern: false,
        emitted_rust_ref_params: None,
        string_ref_string_formal_params: None,
        field_extract_params: None,
        forwarding_borrow_params: None,
    }
}

fn endian_from_bytes(prim: &str, method: &str, width: usize) -> FunctionSignature {
    let arr = byte_array(width);
    FunctionSignature {
        name: format!("{prim}::{method}"),
        param_types: vec![arr.clone()],
        formal_param_types: vec![arr],
        param_ownership: vec![OwnershipMode::Owned],
        return_type: Some(Type::Custom(prim.to_string())),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
        emitted_rust_ref_params: None,
        string_ref_string_formal_params: None,
        field_extract_params: None,
        forwarding_borrow_params: None,
    }
}

const ENDIAN_TO_BYTES: &[&str] = &["to_le_bytes", "to_be_bytes", "to_ne_bytes"];
const ENDIAN_FROM_BYTES: &[&str] = &["from_le_bytes", "from_be_bytes", "from_ne_bytes"];

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
    // Integers and floats: `to_le_bytes` → `[u8; N]` (Copy) and `from_le_bytes`.
    // Untyped results made reuse/index emit `.clone()` on Copy bytes (WDB-464/467).
    for prim in SIGNED
        .iter()
        .chain(UNSIGNED.iter())
        .copied()
        .chain(["f32", "f64"])
    {
        let Some(width) = primitive_endian_width(prim) else {
            continue;
        };
        for method in ENDIAN_TO_BYTES {
            let sig = endian_self_to_bytes(prim, method, width);
            registry.add_function(sig.name.clone(), sig);
        }
        for method in ENDIAN_FROM_BYTES {
            let sig = endian_from_bytes(prim, method, width);
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
        let sig =
            lookup_method_signature("max", Some("i32"), &reg).expect("i32::max boundary signature");
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
        assert_eq!(
            sig.return_type.as_ref(),
            Some(&Type::Custom("usize".into()))
        );
    }

    #[test]
    fn unsigned_has_no_abs() {
        let reg = SignatureRegistry::stdlib();
        assert!(lookup_method_signature("abs", Some("u32"), &reg).is_none());
        assert!(lookup_method_signature("abs", Some("i32"), &reg).is_some());
    }

    #[test]
    fn u32_to_le_bytes_returns_copy_u8_array() {
        let reg = SignatureRegistry::stdlib();
        let sig =
            lookup_method_signature("to_le_bytes", Some("u32"), &reg).expect("u32::to_le_bytes");
        assert_eq!(
            sig.return_type.as_ref(),
            Some(&Type::Array(Box::new(Type::Custom("u8".into())), 4))
        );
        let from = lookup_method_signature("from_le_bytes", Some("f32"), &reg)
            .expect("f32::from_le_bytes");
        assert_eq!(from.return_type.as_ref(), Some(&Type::Custom("f32".into())));
        assert!(!from.has_self_receiver);
        assert_eq!(
            from.param_types.first(),
            Some(&Type::Array(Box::new(Type::Custom("u8".into())), 4))
        );
    }
}
