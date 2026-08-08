//! Stdlib method behavior queries for the analyzer.
//!
//! SignatureRegistry-backed qualified lookups when a receiver type and registry
//! are available; inline name-based fallbacks otherwise (legacy method_registry parity).

use crate::parser::Type;

use super::{FunctionSignature, OwnershipMode, SignatureRegistry};

// ── Inline fallback tables (legacy method_registry parity) ───────────────
// Mutating/readonly-receiver detection is signature-driven (see consensus_*).
// Do not reintroduce method-name lists for ownership decisions.

const MAP_KEY: &[&str] = &["remove", "contains_key", "get", "get_mut", "get_key_value"];

const MAP_TYPES: &[&str] = &["HashMap", "BTreeMap", "Map", "IndexMap"];

const COMMON_STDLIB_NAMES: &[&str] = &[
    "push",
    "pop",
    "insert",
    "remove",
    "clear",
    "get",
    "get_mut",
    "set",
    "len",
    "is_empty",
    "contains",
    "contains_key",
    "first",
    "last",
    "iter",
    "keys",
    "values",
    "clone",
    "to_string",
    "starts_with",
    "ends_with",
    "binary_search",
    "add",
    "to_le_bytes",
    "to_be_bytes",
    "from_le_bytes",
    "from_be_bytes",
];

// ── Inline fallbacks ─────────────────────────────────────────────────────

/// Unqualified fallback: unanimous `MutBorrowed` self across stdlib `::{method}` keys.
/// Prefer [`method_mutates_receiver_qualified`] when receiver type + registry are available.
pub fn method_mutates_receiver(method: &str) -> bool {
    consensus_mutates_receiver(method, SignatureRegistry::stdlib())
}

pub fn is_known_readonly(method: &str) -> bool {
    consensus_readonly_receiver(method, SignatureRegistry::stdlib())
}

pub fn is_map_key_method(method: &str) -> bool {
    MAP_KEY.contains(&method)
}

/// HashSet/BTreeSet lookup method names — AST / trait-identity only
/// (see [`decompose_collection_key_lookup`]). Ownership decisions must use
/// signature `param_ownership` / codegen qualified helpers, not this list.
pub fn is_set_lookup_method(method: &str) -> bool {
    matches!(method, "contains" | "remove")
}

/// Map or set key-lookup method name — for AST decomposition of lookup shapes.
/// Not an ownership oracle (Vec::remove is also named `remove`).
pub fn is_collection_key_method(method: &str) -> bool {
    is_map_key_method(method) || is_set_lookup_method(method)
}

pub(crate) fn is_qualified_map_type(ty: &Type) -> bool {
    match ty {
        Type::Parameterized(base, _) | Type::Custom(base) => {
            is_map_receiver(Some(base.as_str())) && base.contains("::")
        }
        Type::Reference(inner) | Type::MutableReference(inner) => is_qualified_map_type(inner),
        _ => false,
    }
}

/// Map/set lookup syntax may parse as `MethodCall` or `Call(FieldAccess(receiver, method))`.
pub(crate) fn decompose_collection_key_lookup<'ast>(
    expr: &'ast crate::parser::Expression<'ast>,
) -> Option<(
    &'ast crate::parser::Expression<'ast>,
    &'ast str,
    &'ast [(Option<String>, &'ast crate::parser::Expression<'ast>)],
)> {
    match expr {
        crate::parser::Expression::MethodCall {
            object,
            method,
            arguments,
            ..
        } if is_collection_key_method(method) => Some((object, method.as_str(), arguments.as_slice())),
        crate::parser::Expression::Call {
            function,
            arguments,
            ..
        } => match &**function {
            crate::parser::Expression::FieldAccess { object, field, .. }
                if is_collection_key_method(field) =>
            {
                Some((object, field.as_str(), arguments.as_slice()))
            }
            _ => None,
        },
        _ => None,
    }
}

// ── SignatureRegistry helpers ────────────────────────────────────────────

fn lookup_sig<'a>(
    method: &str,
    receiver_type: Option<&str>,
    registry: &'a SignatureRegistry,
) -> Option<&'a FunctionSignature> {
    if let Some(ty) = receiver_type {
        let base = ty.split('<').next().unwrap_or(ty);
        let qualified = format!("{}::{}", base, method);
        if let Some(sig) = registry.get_signature(&qualified) {
            return Some(sig);
        }
        if base != ty {
            let qualified_full = format!("{}::{}", ty, method);
            if let Some(sig) = registry.get_signature(&qualified_full) {
                return Some(sig);
            }
        }
    }
    None
}

fn lookup_unqualified<'a>(
    method: &str,
    registry: &'a SignatureRegistry,
) -> Option<&'a FunctionSignature> {
    if COMMON_STDLIB_NAMES.contains(&method) {
        return None;
    }
    registry.get_signature(method)
}

fn lookup_suffix<'a>(
    method: &str,
    registry: &'a SignatureRegistry,
) -> Option<&'a FunctionSignature> {
    registry.find_unique_signature_ending_with(method)
}

fn return_type_is(sig: &FunctionSignature, pred: impl Fn(&Type) -> bool) -> bool {
    sig.return_type.as_ref().is_some_and(pred)
}

fn first_arg_ownership(sig: &FunctionSignature) -> Option<OwnershipMode> {
    let start = if sig.has_self_receiver { 1 } else { 0 };
    sig.param_ownership.get(start).copied()
}

fn first_arg_type(sig: &FunctionSignature) -> Option<&Type> {
    let start = if sig.has_self_receiver { 1 } else { 0 };
    sig.param_types.get(start)
}

fn arg_count(sig: &FunctionSignature) -> usize {
    if sig.has_self_receiver {
        sig.param_ownership.len().saturating_sub(1)
    } else {
        sig.param_ownership.len()
    }
}

fn is_reference_type(ty: &Type) -> bool {
    matches!(ty, Type::Reference(_))
}

fn is_str_reference(ty: &Type) -> bool {
    matches!(ty, Type::Reference(inner) if matches!(&**inner, Type::Custom(n) if n == "str"))
}

fn is_usize_type(ty: &Type) -> bool {
    matches!(ty, Type::Custom(n) if n == "usize")
}

fn is_closure_type(ty: &Type) -> bool {
    matches!(ty, Type::Custom(n) if n == "Fn" || n == "FnMut" || n == "FnOnce")
        || matches!(ty, Type::FunctionPointer { .. })
}

pub(crate) fn is_map_receiver(receiver_type: Option<&str>) -> bool {
    receiver_type.is_some_and(|ty| {
        let base = ty.split('<').next().unwrap_or(ty);
        let short = base.rsplit("::").next().unwrap_or(base);
        MAP_TYPES.contains(&short)
    })
}

fn sig_mutates_receiver(sig: &FunctionSignature) -> bool {
    sig.has_self_receiver
        && matches!(
            sig.param_ownership.first(),
            Some(OwnershipMode::MutBorrowed)
        )
}

/// True when the receiver is not `&mut self` (`Owned` and `Borrowed` both count).
fn sig_readonly_receiver(sig: &FunctionSignature) -> bool {
    sig.has_self_receiver
        && sig
            .param_ownership
            .first()
            .is_some_and(|o| *o != OwnershipMode::MutBorrowed)
}

/// When `::{method}` is registered on many types, true only if every *instance*
/// method match takes `&mut self`. Free functions that share the suffix are
/// ignored. Ambiguous method names return false — use the qualified API instead.
fn consensus_mutates_receiver(method: &str, registry: &SignatureRegistry) -> bool {
    let pattern = format!("::{method}");
    let mut any = false;
    for (key, sig) in registry.all_signatures_for_suffix_search() {
        if key.ends_with(&pattern) {
            if !sig.has_self_receiver {
                continue;
            }
            any = true;
            if !sig_mutates_receiver(sig) {
                return false;
            }
        }
    }
    any
}

/// When `::{method}` is registered on many types, true only if every *instance*
/// method match does not take `&mut self` (`Owned` and `Borrowed` both count as
/// non-mutating-in-place). Free functions that share the suffix are ignored.
fn consensus_readonly_receiver(method: &str, registry: &SignatureRegistry) -> bool {
    let pattern = format!("::{method}");
    let mut any = false;
    for (key, sig) in registry.all_signatures_for_suffix_search() {
        if key.ends_with(&pattern) {
            if !sig.has_self_receiver {
                continue;
            }
            any = true;
            if !sig_readonly_receiver(sig) {
                return false;
            }
        }
    }
    any
}

pub fn method_mutates_receiver_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    if let Some(sig) = lookup_sig(method, receiver_type, registry) {
        return sig_mutates_receiver(sig);
    }
    if let Some(sig) = lookup_suffix(method, registry) {
        return sig_mutates_receiver(sig);
    }
    if let Some(sig) = lookup_unqualified(method, registry) {
        return sig_mutates_receiver(sig);
    }
    consensus_mutates_receiver(method, registry)
}

pub fn is_known_readonly_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    if let Some(sig) = lookup_sig(method, receiver_type, registry) {
        return sig_readonly_receiver(sig);
    }
    if let Some(sig) = lookup_suffix(method, registry) {
        return sig_readonly_receiver(sig);
    }
    if let Some(sig) = lookup_unqualified(method, registry) {
        return sig_readonly_receiver(sig);
    }
    consensus_readonly_receiver(method, registry)
}

fn return_type_is_self(sig: &FunctionSignature) -> bool {
    return_type_is(sig, |ty| matches!(ty, Type::Custom(n) if n == "Self"))
}

/// When `::{method}` is registered on many types, true only if every match returns `Self`.
fn consensus_return_is_self(method: &str, registry: &SignatureRegistry) -> bool {
    let pattern = format!("::{method}");
    let mut any = false;
    for (key, sig) in registry.all_signatures_for_suffix_search() {
        if key.ends_with(&pattern) {
            any = true;
            if !return_type_is_self(sig) {
                return false;
            }
        }
    }
    any
}

/// Is this method type-preserving (return type == `Self`)?
/// Driven by resolved signatures (and consensus when the receiver type is unknown).
pub fn method_is_type_preserving_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    if let Some(sig) = lookup_sig(method, receiver_type, registry) {
        return return_type_is_self(sig);
    }
    if let Some(sig) = lookup_suffix(method, registry) {
        return return_type_is_self(sig);
    }
    consensus_return_is_self(method, registry)
}

/// True when `Option::{method}` takes owned `self` (needs `.as_ref()` desugar under borrow).
pub fn option_owned_self_method(method: &str, registry: &SignatureRegistry) -> bool {
    let Some(sig) = lookup_sig(method, Some("Option"), registry) else {
        return false;
    };
    sig.has_self_receiver
        && matches!(sig.param_ownership.first(), Some(OwnershipMode::Owned))
}

/// Peel `&T` / `&mut T` and return `"f32"` / `"f64"` for primitive float receivers.
pub fn float_primitive_name(ty: &Type) -> Option<&'static str> {
    match ty {
        Type::Float => Some("f64"),
        Type::Custom(s) if s == "float" => Some("f64"),
        Type::Custom(s) if s == "f32" => Some("f32"),
        Type::Custom(s) if s == "f64" => Some("f64"),
        Type::Reference(inner) | Type::MutableReference(inner) => float_primitive_name(inner),
        _ => None,
    }
}

fn return_type_is_float(sig: &FunctionSignature, float_name: &str) -> bool {
    sig.return_type.as_ref().is_some_and(|t| match t {
        Type::Float => float_name == "f64",
        Type::Custom(s) if s == float_name => true,
        _ => false,
    })
}

/// True when `receiver` is `f32`/`f64` and `Receiver::method` returns the same float type.
///
/// Guards struct builder methods (`Slider::max`) — receiver must be a primitive float, not
/// an arbitrary type that happens to define `max`.
pub fn method_preserves_float_receiver(
    method: &str,
    receiver_type: Option<&Type>,
    registry: &SignatureRegistry,
) -> bool {
    let Some(float_name) = receiver_type.and_then(float_primitive_name) else {
        return false;
    };
    lookup_sig(method, Some(float_name), registry).is_some_and(|sig| {
        return_type_is_float(sig, float_name)
    })
}

/// True when the resolved float method has at least one parameter of the receiver float type
/// (e.g. `f32::max(other: f32)`). Used for float arg/receiver unification constraints.
pub fn method_float_args_match_receiver(
    method: &str,
    receiver_type: Option<&Type>,
    registry: &SignatureRegistry,
) -> bool {
    let Some(float_name) = receiver_type.and_then(float_primitive_name) else {
        return false;
    };
    let Some(sig) = lookup_sig(method, Some(float_name), registry) else {
        return false;
    };
    if !return_type_is_float(sig, float_name) {
        return false;
    }
    let float_ty = Type::Custom(float_name.to_string());
    let start = if sig.has_self_receiver { 1 } else { 0 };
    sig.param_types
        .get(start..)
        .is_some_and(|params| params.iter().any(|p| *p == float_ty))
}

pub fn method_is_storage_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig = lookup_sig(method, receiver_type, registry)
        .or_else(|| lookup_unqualified(method, registry));
    if let Some(s) = sig {
        if !s.has_self_receiver || arg_count(s) == 0 {
            return false;
        }
        if s.param_ownership[0] != OwnershipMode::MutBorrowed {
            return false;
        }
        let start = 1;
        for i in start..s.param_ownership.len() {
            if s.param_ownership[i] == OwnershipMode::Owned {
                if let Some(ty) = s.param_types.get(i) {
                    if !is_usize_type(ty) && !is_closure_type(ty) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub fn method_is_map_key_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    if !is_map_receiver(receiver_type) {
        if receiver_type.is_some() {
            return false;
        }
        for map_ty in MAP_TYPES {
            if let Some(sig) = lookup_sig(method, Some(map_ty), registry) {
                if sig.has_self_receiver
                    && first_arg_ownership(sig) == Some(OwnershipMode::Borrowed)
                {
                    if first_arg_type(sig).is_some_and(is_reference_type) {
                        return true;
                    }
                }
            }
        }
        return false;
    }
    let sig = lookup_sig(method, receiver_type, registry);
    sig.is_some_and(|s| {
        s.has_self_receiver
            && first_arg_ownership(s) == Some(OwnershipMode::Borrowed)
            && first_arg_type(s).is_some_and(is_reference_type)
    })
}

pub fn method_is_slice_search_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig = lookup_sig(method, receiver_type, registry)
        .or_else(|| lookup_unqualified(method, registry));
    sig.is_some_and(|s| {
        s.has_self_receiver
            && first_arg_ownership(s) == Some(OwnershipMode::Borrowed)
            && first_arg_type(s).is_some_and(|ty| is_reference_type(ty) && !is_str_reference(ty))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_mutates_receiver_stdlib_consensus() {
        assert!(method_mutates_receiver("push"));
        assert!(method_mutates_receiver("clear"));
        assert!(!method_mutates_receiver("len"));
        assert!(
            !method_mutates_receiver("replace"),
            "Option vs String replace must not consensus-mutate"
        );
        assert!(!method_mutates_receiver("no_such_method_zzz"));
    }

    #[test]
    fn method_mutates_receiver_qualified_option_replace() {
        let reg = SignatureRegistry::stdlib();
        assert!(method_mutates_receiver_qualified(
            "replace",
            Some("Option"),
            reg
        ));
        assert!(!method_mutates_receiver_qualified(
            "replace",
            Some("String"),
            reg
        ));
    }

    #[test]
    fn is_known_readonly_stdlib_consensus() {
        assert!(is_known_readonly("len"));
        assert!(is_known_readonly("is_empty"));
        assert!(is_known_readonly("get"));
        assert!(!is_known_readonly("push"));
        assert!(!is_known_readonly("clear"));
        assert!(
            !is_known_readonly("replace"),
            "Option vs String replace must not consensus-readonly"
        );
        assert!(!is_known_readonly("no_such_method_zzz"));
    }

    #[test]
    fn consensus_readonly_receiver_empty_registry_is_false() {
        let empty = SignatureRegistry::empty();
        assert!(!consensus_readonly_receiver("len", &empty));
        assert!(!is_known_readonly_qualified("len", Some("Vec"), &empty));
    }

    #[test]
    fn consensus_readonly_receiver_mixed_ownership_is_false() {
        let mut reg = SignatureRegistry::empty();
        let mut a = FunctionSignature::default();
        a.name = "A::peek".into();
        a.param_types = vec![Type::Custom("A".into())];
        a.param_ownership = vec![OwnershipMode::MutBorrowed];
        a.has_self_receiver = true;
        reg.add_function("A::peek".into(), a);

        let mut b = FunctionSignature::default();
        b.name = "B::peek".into();
        b.param_types = vec![Type::Custom("B".into())];
        b.param_ownership = vec![OwnershipMode::Borrowed];
        b.has_self_receiver = true;
        reg.add_function("B::peek".into(), b);

        assert!(!consensus_readonly_receiver("peek", &reg));
        assert!(!is_known_readonly_qualified("peek", Some("A"), &reg));
        assert!(is_known_readonly_qualified("peek", Some("B"), &reg));
    }

    #[test]
    fn consensus_readonly_receiver_unanimous_borrowed_is_true() {
        let mut reg = SignatureRegistry::empty();
        for (ty, own) in [
            ("A", OwnershipMode::Borrowed),
            ("B", OwnershipMode::Borrowed),
        ] {
            let mut sig = FunctionSignature::default();
            sig.name = format!("{ty}::size");
            sig.param_types = vec![Type::Custom(ty.into())];
            sig.param_ownership = vec![own];
            sig.has_self_receiver = true;
            reg.add_function(format!("{ty}::size"), sig);
        }
        assert!(consensus_readonly_receiver("size", &reg));
    }
}
