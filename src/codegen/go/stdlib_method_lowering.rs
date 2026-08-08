//! Signature-driven method lowering helpers for the Go backend.

use crate::analyzer::{SignatureRegistry, stdlib_method_traits};
use crate::parser::Type;

fn lookup_method_sig<'a>(
    method: &str,
    receiver_type: Option<&str>,
    registry: &'a SignatureRegistry,
) -> Option<&'a crate::analyzer::FunctionSignature> {
    if let Some(ty) = receiver_type {
        let base = ty.split('<').next().unwrap_or(ty);
        if let Some(sig) = registry.get_signature(&format!("{base}::{method}")) {
            return Some(sig);
        }
    }
    registry.find_unique_signature_ending_with(method)
}

/// True when `receiver.method(arg)` should lower to Go `append(receiver, arg)`.
///
/// Requires a mutating storage method (`Vec::push`-like): `&mut self` receiver and an owned
/// element parameter. Without a receiver type, only unambiguous `Vec::push` matches.
pub fn go_lowers_push_to_append(method: &str, receiver_type: Option<&str>) -> bool {
    let reg = SignatureRegistry::stdlib();
    if let Some(ty) = receiver_type {
        if stdlib_method_traits::method_is_storage_qualified(method, Some(ty), reg) {
            return true;
        }
    }
    stdlib_method_traits::method_is_storage_qualified(method, Some("Vec"), reg)
}

/// True when the call is a stdlib membership test we cannot lower yet (emit stub).
pub fn go_contains_needs_stub(method: &str, receiver_type: Option<&str>) -> bool {
    let reg = SignatureRegistry::stdlib();
    let sig = lookup_method_sig(method, receiver_type, reg);
    sig.is_some_and(|s| {
        s.return_type.as_ref().is_some_and(|t| matches!(t, Type::Bool))
            && s.param_ownership.len() > if s.has_self_receiver { 1 } else { 0 }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec_push_lowers_to_append() {
        assert!(go_lowers_push_to_append("push", Some("Vec")));
    }

    #[test]
    fn string_push_is_not_append() {
        assert!(!go_lowers_push_to_append("push", Some("String")));
    }
}
