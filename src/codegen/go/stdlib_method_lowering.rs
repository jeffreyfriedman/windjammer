//! Signature-driven method lowering helpers for the Go backend.

use crate::analyzer::{SignatureRegistry, stdlib_method_traits};

fn is_string_like_receiver_base(base: &str) -> bool {
    matches!(base, "String" | "string" | "str")
}

fn receiver_base(ty: &str) -> &str {
    ty.split('<').next().unwrap_or(ty)
}

/// Consensus for Go `append` lowering: storage-like and not homonymous with String APIs.
fn consensus_go_append_storage(method: &str, registry: &SignatureRegistry) -> bool {
    let mut any_non_string = false;
    let mut saw_string = false;
    for (key, sig) in registry.signatures_for_method_name(method) {
        if !sig.has_self_receiver {
            continue;
        }
        let base = key.split("::").next().unwrap_or("");
        if is_string_like_receiver_base(base) {
            saw_string = true;
            continue;
        }
        any_non_string = true;
        if !stdlib_method_traits::method_is_storage_qualified(method, Some(base), registry) {
            return false;
        }
    }
    if saw_string && any_non_string {
        return false;
    }
    any_non_string
}

/// True when `receiver.method(arg)` should lower to Go `append(receiver, arg)`.
///
/// Requires a mutating storage method (`Vec::push`-like): `&mut self` receiver and an owned
/// element parameter. `String::push` is excluded — it is storage-like but not slice append.
/// Without a receiver type, returns false when the name is ambiguous (e.g. homonymous `push`).
pub fn go_lowers_push_to_append(method: &str, receiver_type: Option<&str>) -> bool {
    let reg = SignatureRegistry::stdlib();
    match receiver_type {
        Some(ty) if is_string_like_receiver_base(receiver_base(ty)) => false,
        Some(ty) => stdlib_method_traits::method_is_storage_qualified(method, Some(ty), reg),
        None => consensus_go_append_storage(method, reg),
    }
}

/// True when the call is a stdlib membership test we cannot lower yet (emit stub).
pub fn go_contains_needs_stub(method: &str, receiver_type: Option<&str>) -> bool {
    let reg = SignatureRegistry::stdlib();
    stdlib_method_traits::method_is_membership_test_qualified(method, receiver_type, reg)
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

    #[test]
    fn push_without_receiver_type_is_ambiguous() {
        assert!(!go_lowers_push_to_append("push", None));
    }
}
