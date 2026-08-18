//! Stdlib method behavior queries for the analyzer.
//!
//! SignatureRegistry-backed qualified lookups when a receiver type and registry
//! are available; inline name-based fallbacks otherwise (legacy method_registry parity).

use crate::parser::Type;

use super::{FunctionSignature, OwnershipMode, SignatureRegistry};

// ── Inline fallback tables (legacy method_registry parity) ───────────────
// Mutating/readonly-receiver detection is signature-driven (see consensus_*).
// Do not reintroduce method-name lists for ownership decisions.

const MAP_TYPES: &[&str] = &["HashMap", "BTreeMap", "Map", "IndexMap"];
const SET_TYPES: &[&str] = &["HashSet", "BTreeSet"];

/// Receiver type names to try for `Type::method` registry lookup.
///
/// Handles generics (`HashMap<K,V>`), module prefixes (`std::collections::HashMap`),
/// and stdlib aliases (`Map` → `HashMap`, `string` → `String`).
pub(crate) fn stdlib_receiver_lookup_candidates(receiver_type: &str) -> Vec<String> {
    let base = receiver_type.split('<').next().unwrap_or(receiver_type);
    let leaf = base.rsplit("::").next().unwrap_or(base);
    let mut out = Vec::new();
    let mut push = |s: &str| {
        if !s.is_empty() && !out.iter().any(|x| x == s) {
            out.push(s.to_string());
        }
    };
    push(receiver_type);
    push(base);
    push(leaf);
    if matches!(leaf, "str" | "string" | "String") {
        push("String");
    }
    if matches!(leaf, "Map") {
        push("HashMap");
    }
    out
}

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
    method_is_map_key_qualified(method, None, SignatureRegistry::stdlib())
}

/// HashSet/BTreeSet lookup — signature-driven (borrowed first arg on set types).
/// Ownership decisions must use `param_ownership` / codegen qualified helpers.
pub fn is_set_lookup_method(method: &str) -> bool {
    method_is_set_lookup_qualified(method, None, SignatureRegistry::stdlib())
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
        } if is_collection_key_method(method) => {
            Some((object, method.as_str(), arguments.as_slice()))
        }
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

pub(crate) fn lookup_method_signature<'a>(
    method: &str,
    receiver_type: Option<&str>,
    registry: &'a SignatureRegistry,
) -> Option<&'a FunctionSignature> {
    lookup_sig(method, receiver_type, registry)
}

fn lookup_sig<'a>(
    method: &str,
    receiver_type: Option<&str>,
    registry: &'a SignatureRegistry,
) -> Option<&'a FunctionSignature> {
    if let Some(ty) = receiver_type {
        for candidate in stdlib_receiver_lookup_candidates(ty) {
            let qualified = format!("{}::{}", candidate, method);
            if let Some(sig) = registry.get_signature(&qualified) {
                // Declaration stubs (empty ownership) must not shadow stdlib
                // `Type::method` keys — continue so callers can consult fallback.
                if sig.param_ownership.is_empty() {
                    continue;
                }
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
    if registry.has_method_name_collision(method) {
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

/// Look up a free function or `Type::method` without hardcoding type names.
///
/// Order: qualified receiver → exact key → unique `::{name}` suffix.
pub(crate) fn lookup_callable_signature<'a>(
    name: &str,
    receiver_type: Option<&str>,
    registry: &'a SignatureRegistry,
) -> Option<&'a FunctionSignature> {
    let simple = name.rsplit("::").next().unwrap_or(name);
    if let Some(sig) = lookup_sig(simple, receiver_type, registry) {
        return Some(sig);
    }
    if let Some(sig) = registry.get_signature(name) {
        if !sig.param_ownership.is_empty() {
            return Some(sig);
        }
    }
    if name != simple {
        if !registry.has_collision(simple) {
            if let Some(sig) = registry.get_signature(simple) {
                if !sig.param_ownership.is_empty() {
                    return Some(sig);
                }
            }
        }
    }
    lookup_suffix(simple, registry)
}

/// Whether call-site argument `arg_idx` is `&mut`.
///
/// `args_include_receiver` is true for `Call` (UFCS / free functions: every
/// source argument is in the list). Method calls omit `self`, so pass false.
pub(crate) fn callable_arg_expects_mut_borrow(
    name: &str,
    receiver_type: Option<&str>,
    arg_idx: usize,
    args_include_receiver: bool,
    registry: &SignatureRegistry,
) -> bool {
    let Some(sig) = lookup_callable_signature(name, receiver_type, registry) else {
        return false;
    };
    let mode = if args_include_receiver {
        sig.param_ownership.get(arg_idx)
    } else {
        sig.param_ownership_for_arg(arg_idx)
    };
    mode.is_some_and(|o| *o == OwnershipMode::MutBorrowed)
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

fn is_set_receiver(receiver_type: Option<&str>) -> bool {
    receiver_type.is_some_and(|ty| {
        let base = ty.split('<').next().unwrap_or(ty);
        let short = base.rsplit("::").next().unwrap_or(base);
        SET_TYPES.contains(&short)
    })
}

fn method_matches_borrowed_key_on_types(
    method: &str,
    receiver_type: Option<&str>,
    type_names: &[&str],
    is_receiver: impl Fn(Option<&str>) -> bool,
    registry: &SignatureRegistry,
) -> bool {
    if !is_receiver(receiver_type) {
        if receiver_type.is_some() {
            return false;
        }
        for ty in type_names {
            if let Some(sig) = lookup_sig(method, Some(ty), registry) {
                if sig.has_self_receiver
                    && first_arg_ownership(sig) == Some(OwnershipMode::Borrowed)
                    && first_arg_type(sig).is_some_and(is_reference_type)
                {
                    return true;
                }
            }
        }
        return false;
    }
    lookup_sig(method, receiver_type, registry).is_some_and(|s| {
        s.has_self_receiver
            && first_arg_ownership(s) == Some(OwnershipMode::Borrowed)
            && first_arg_type(s).is_some_and(is_reference_type)
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
    if receiver_type.is_some() {
        // Known receiver: only that type's signature (local, then stdlib).
        // Never a unique `::{method}` from a *different* type.
        // Incomplete / associated-fn stubs (no `&self`/`&mut self`) must not
        // shadow the stdlib instance method.
        if let Some(sig) = lookup_sig(method, receiver_type, registry) {
            if sig.has_self_receiver {
                return sig_mutates_receiver(sig);
            }
        }
        if let Some(sig) = lookup_sig(method, receiver_type, SignatureRegistry::stdlib()) {
            if sig.has_self_receiver {
                return sig_mutates_receiver(sig);
            }
        }
        return false;
    }
    if let Some(sig) = lookup_suffix(method, registry) {
        if sig.has_self_receiver {
            return sig_mutates_receiver(sig);
        }
    }
    if let Some(sig) = lookup_unqualified(method, registry) {
        if sig.has_self_receiver {
            return sig_mutates_receiver(sig);
        }
    }
    if consensus_mutates_receiver(method, registry) {
        return true;
    }
    consensus_mutates_receiver(method, SignatureRegistry::stdlib())
}

/// Signature-driven mutation check for `receiver.method()`.
///
/// When `receiver_type_base` is `Some`, only that type's signature counts (no
/// unqualified stdlib consensus). When `None`, uses unique qualified match then
/// suffix/unqualified consensus. Mixed-ownership names (e.g. `replace`) do not
/// consensus-mutate.
pub fn method_call_mutates_receiver(
    method: &str,
    receiver_type_base: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    method_mutates_receiver_qualified(method, receiver_type_base, registry)
}

fn sig_consumes_receiver(sig: &FunctionSignature) -> bool {
    sig.has_self_receiver && matches!(sig.param_ownership.first(), Some(OwnershipMode::Owned))
}

/// True when the method takes owned `self` (consumes the receiver).
pub fn method_call_consumes_receiver(
    method: &str,
    receiver_type_base: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    if let Some(ty) = receiver_type_base {
        if let Some(sig) = lookup_sig(method, Some(ty), registry) {
            if sig.has_self_receiver {
                return sig_consumes_receiver(sig);
            }
        }
        if let Some(sig) = lookup_sig(method, Some(ty), SignatureRegistry::stdlib()) {
            if sig.has_self_receiver {
                return sig_consumes_receiver(sig);
            }
        }
        return false;
    }
    if let Some(sig) = lookup_suffix(method, registry) {
        if sig.has_self_receiver && sig_consumes_receiver(sig) {
            return true;
        }
    }
    if let Some(sig) = lookup_unqualified(method, registry) {
        if sig.has_self_receiver {
            return sig_consumes_receiver(sig);
        }
    }
    false
}

pub fn is_known_readonly_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    if receiver_type.is_some() {
        if let Some(sig) = lookup_sig(method, receiver_type, registry) {
            return sig_readonly_receiver(sig);
        }
        if let Some(sig) = lookup_sig(method, receiver_type, SignatureRegistry::stdlib()) {
            return sig_readonly_receiver(sig);
        }
        return false;
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
    sig.has_self_receiver && matches!(sig.param_ownership.first(), Some(OwnershipMode::Owned))
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
    lookup_sig(method, Some(float_name), registry)
        .is_some_and(|sig| return_type_is_float(sig, float_name))
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

fn sig_is_storage_like(sig: &FunctionSignature) -> bool {
    if !sig.has_self_receiver || arg_count(sig) == 0 {
        return false;
    }
    if sig.param_ownership[0] != OwnershipMode::MutBorrowed {
        return false;
    }
    let start = 1;
    for i in start..sig.param_ownership.len() {
        if sig.param_ownership[i] == OwnershipMode::Owned {
            if let Some(ty) = sig.param_types.get(i) {
                if !is_usize_type(ty) && !is_closure_type(ty) {
                    return true;
                }
            }
        }
    }
    false
}

/// When `::{method}` is registered on many types, true only if every *instance*
/// method match is storage-like (`Vec::push`-style). Ambiguous names return false.
pub fn consensus_storage_method(method: &str, registry: &SignatureRegistry) -> bool {
    let pattern = format!("::{method}");
    let mut any = false;
    for (key, sig) in registry.all_signatures_for_suffix_search() {
        if key.ends_with(&pattern) {
            if !sig.has_self_receiver {
                continue;
            }
            any = true;
            if !sig_is_storage_like(sig) {
                return false;
            }
        }
    }
    any
}

fn sig_is_closure_taking(sig: &FunctionSignature) -> bool {
    first_arg_type(sig).is_some_and(is_closure_type)
}

/// Iterator/adapter protocol: the predicate/visitor closure receives `&T`
/// (not owned `T`). Distinct from `map`/`flat_map`/`filter_map`, which consume
/// owned elements. This is language-level iterator semantics, not call-site
/// argument ownership (see no-hardcoded-method-names exception for closure params).
pub fn method_predicate_closure_receives_ref(method: &str) -> bool {
    matches!(
        method,
        "retain"
            | "filter"
            | "any"
            | "all"
            | "find"
            | "position"
            | "rposition"
            | "take_while"
            | "skip_while"
            | "partition"
            | "inspect"
    )
}

/// Unanimous closure-first-arg across all stdlib instance methods named `::{method}`.
pub fn consensus_closure_taking_method(method: &str, registry: &SignatureRegistry) -> bool {
    let pattern = format!("::{method}");
    let mut any = false;
    for (key, sig) in registry.all_signatures_for_suffix_search() {
        if key.ends_with(&pattern) {
            if !sig.has_self_receiver {
                continue;
            }
            any = true;
            if !sig_is_closure_taking(sig) {
                return false;
            }
        }
    }
    any
}

fn sig_is_membership_test(sig: &FunctionSignature) -> bool {
    sig.has_self_receiver
        && sig
            .return_type
            .as_ref()
            .is_some_and(|t| matches!(t, Type::Bool))
        && first_arg_ownership(sig) == Some(OwnershipMode::Borrowed)
        && first_arg_type(sig).is_some_and(is_reference_type)
}

/// Unanimous bool membership-test shape across stdlib instance `::{method}` keys.
pub fn consensus_membership_test_method(method: &str, registry: &SignatureRegistry) -> bool {
    let pattern = format!("::{method}");
    let mut any = false;
    for (key, sig) in registry.all_signatures_for_suffix_search() {
        if key.ends_with(&pattern) {
            if !sig.has_self_receiver {
                continue;
            }
            any = true;
            if !sig_is_membership_test(sig) {
                return false;
            }
        }
    }
    any
}

pub fn method_is_storage_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig = lookup_sig(method, receiver_type, registry)
        .or_else(|| lookup_unqualified(method, registry));
    sig.is_some_and(sig_is_storage_like)
}

pub fn method_is_closure_taking_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    if let Some(recv) = receiver_type {
        if let Some(sig) = lookup_sig(method, Some(recv), registry) {
            return sig_is_closure_taking(sig);
        }
        // Collection iterator adapters: `Vec::find` isn't registered; `Iterator::find` is.
        // Homonym `String::find(char)` is resolved above and returns false for fn-ptr args.
        let stdlib = SignatureRegistry::stdlib();
        if lookup_sig(method, Some("Iterator"), &stdlib).is_some_and(sig_is_closure_taking) {
            return true;
        }
        return lookup_unqualified(method, registry).is_some_and(sig_is_closure_taking);
    }
    consensus_closure_taking_method(method, registry)
}

/// String search methods (`starts_with`, `ends_with`, `contains`) whose first arg is `&str`.
pub fn method_is_string_search_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig = lookup_sig(method, receiver_type, registry)
        .or_else(|| lookup_unqualified(method, registry));
    sig.is_some_and(|s| {
        s.has_self_receiver
            && first_arg_ownership(s) == Some(OwnershipMode::Borrowed)
            && first_arg_type(s).is_some_and(is_str_reference)
    })
}

fn return_type_is_string_like(sig: &FunctionSignature) -> bool {
    fn type_is_string_like(t: &Type) -> bool {
        match t {
            Type::String => true,
            Type::Custom(n) => n == "string" || n == "str",
            _ => false,
        }
    }
    sig.return_type.as_ref().is_some_and(|t| {
        type_is_string_like(t) || matches!(t, Type::Option(inner) if type_is_string_like(inner))
    })
}

/// True when `method` is a String/`strings` runtime API (search, transform, module fn).
pub fn method_is_string_runtime_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    if method_is_string_search_qualified(method, receiver_type, registry) {
        return true;
    }
    if registry
        .get_signature(&format!("strings::{method}"))
        .is_some()
    {
        return true;
    }
    for ty in ["String", "string", "str"] {
        if let Some(sig) = lookup_sig(method, Some(ty), registry) {
            if sig.has_self_receiver && return_type_is_string_like(sig) {
                return true;
            }
        }
    }
    false
}

pub fn method_is_membership_test_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    if method_is_string_search_qualified(method, receiver_type, registry) {
        return true;
    }
    let sig = lookup_sig(method, receiver_type, registry)
        .or_else(|| lookup_unqualified(method, registry));
    if let Some(s) = sig {
        return sig_is_membership_test(s);
    }
    if receiver_type.is_none() {
        return consensus_membership_test_method(method, registry);
    }
    false
}

pub fn method_is_map_key_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    method_matches_borrowed_key_on_types(
        method,
        receiver_type,
        MAP_TYPES,
        is_map_receiver,
        registry,
    )
}

pub fn method_is_set_lookup_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    method_matches_borrowed_key_on_types(
        method,
        receiver_type,
        SET_TYPES,
        is_set_receiver,
        registry,
    )
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
    fn map_and_set_key_methods_are_signature_driven() {
        assert!(is_map_key_method("get"));
        assert!(is_map_key_method("contains_key"));
        assert!(!is_map_key_method("push"));
        assert!(!is_map_key_method("insert"));
        assert!(is_set_lookup_method("contains"));
        assert!(is_set_lookup_method("remove"));
        assert!(!is_set_lookup_method("insert"));
    }

    #[test]
    fn method_call_mutates_receiver_qualified_user_type_no_consensus_poison() {
        let mut reg = SignatureRegistry::empty();
        let mut sig = FunctionSignature::default();
        sig.name = "Section::render".into();
        sig.param_ownership = vec![OwnershipMode::Borrowed];
        sig.has_self_receiver = true;
        reg.add_function("Section::render".into(), sig);
        assert!(!method_call_mutates_receiver(
            "render",
            Some("Section"),
            &reg
        ));
        assert!(!method_call_mutates_receiver("render", None, &reg));
    }

    #[test]
    fn method_call_mutates_hashmap_remove_with_qualified_path() {
        let reg = SignatureRegistry::stdlib();
        assert!(method_call_mutates_receiver("remove", Some("HashMap"), reg));
        assert!(method_call_mutates_receiver(
            "remove",
            Some("std::collections::HashMap"),
            reg
        ));
        assert!(method_call_mutates_receiver(
            "remove",
            Some("HashMap<i32, Transform>"),
            reg
        ));
        assert!(method_call_mutates_receiver("push", Some("Vec"), reg));
        assert!(method_call_mutates_receiver(
            "push",
            Some("alloc::vec::Vec<i32>"),
            reg
        ));
    }

    #[test]
    fn lookup_callable_does_not_hardcode_type_names() {
        let mut reg = SignatureRegistry::empty();
        let mut sig = FunctionSignature::default();
        sig.name = "Offer::can_take".into();
        sig.param_ownership = vec![OwnershipMode::Borrowed, OwnershipMode::MutBorrowed];
        sig.has_self_receiver = true;
        reg.add_function("Offer::can_take".into(), sig);

        assert!(lookup_callable_signature("can_take", Some("Offer"), &reg).is_some());
        assert!(lookup_callable_signature("can_take", None, &reg).is_some());
        assert!(callable_arg_expects_mut_borrow(
            "can_take",
            Some("Offer"),
            0,
            false,
            &reg
        ));
        assert!(!callable_arg_expects_mut_borrow(
            "can_take",
            Some("Offer"),
            0,
            true,
            &reg
        ));
    }

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
        // Known stdlib type still resolves via stdlib fallback, not empty-registry consensus.
        assert!(is_known_readonly_qualified("len", Some("Vec"), &empty));
        assert!(!is_known_readonly_qualified("len", Some("Writer"), &empty));
    }

    #[test]
    fn qualified_lookup_uses_stdlib_when_local_registry_empty() {
        let empty = SignatureRegistry::empty();
        assert!(method_mutates_receiver_qualified(
            "push",
            Some("Vec"),
            &empty
        ));
        assert!(!method_mutates_receiver_qualified(
            "len",
            Some("Vec"),
            &empty
        ));
        assert!(!method_mutates_receiver_qualified(
            "append",
            Some("Writer"),
            &empty
        ));
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
