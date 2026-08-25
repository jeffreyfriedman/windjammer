//! Signature-driven method behavior queries.
//!
//! Every query first attempts a type-qualified lookup in `SignatureRegistry`
//! (e.g. `Vec::push`), deriving the answer from `FunctionSignature` fields.
//! For non-derivable behaviors (strip-redundant, desugar, ambiguity guards),
//! small const tables live in `rust_stdlib_annotations`.

use crate::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use crate::parser::{Expression, Type};

pub use crate::analyzer::stdlib_method_traits::{
    float_primitive_name, method_float_args_match_receiver, method_preserves_float_receiver,
};

// ── Helpers ──────────────────────────────────────────────────────────────

/// Receiver type names to try for stdlib `Type::method` registry lookup.
///
/// Windjammer `string`/`str`/`&str` receivers lower to Rust `&str` at call sites,
/// but stdlib metadata registers text methods on `String` (e.g. `String::find`).
pub(crate) fn stdlib_receiver_lookup_candidates(receiver_type: &str) -> Vec<String> {
    crate::analyzer::stdlib_method_traits::stdlib_receiver_lookup_candidates(receiver_type)
}

/// Attempt a type-qualified signature lookup, trying multiple receiver type
/// representations (e.g. `Vec`, `Vec<T>`, bare generic base).
fn lookup_sig<'a>(
    method: &str,
    receiver_type: Option<&str>,
    registry: &'a SignatureRegistry,
) -> Option<&'a FunctionSignature> {
    crate::analyzer::stdlib_method_traits::lookup_method_signature(method, receiver_type, registry)
}

/// Suffix lookup fallback: finds signatures registered as `Type::method`
/// when the receiver type is unknown. Only returns a result when there is
/// exactly one candidate — if multiple types define the same method name,
/// we cannot disambiguate and must use default behavior.
fn lookup_suffix<'a>(
    method: &str,
    registry: &'a SignatureRegistry,
) -> Option<&'a FunctionSignature> {
    registry.find_unique_signature_ending_with(method)
}

/// Get the first non-self parameter ownership mode.
fn first_arg_ownership(sig: &FunctionSignature) -> Option<OwnershipMode> {
    let start = if sig.has_self_receiver { 1 } else { 0 };
    sig.param_ownership.get(start).copied()
}

/// Get the first non-self parameter type.
fn first_arg_type(sig: &FunctionSignature) -> Option<&Type> {
    let start = if sig.has_self_receiver { 1 } else { 0 };
    sig.param_types.get(start)
}

/// Number of non-self parameters.
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

fn is_closure_type(ty: &Type) -> bool {
    matches!(ty, Type::Custom(n) if n == "Fn" || n == "FnMut" || n == "FnOnce")
        || matches!(ty, Type::FunctionPointer { .. })
}

fn is_usize_type(ty: &Type) -> bool {
    matches!(ty, Type::Custom(n) if n == "usize")
}

fn return_type_is(sig: &FunctionSignature, pred: impl Fn(&Type) -> bool) -> bool {
    sig.return_type.as_ref().is_some_and(&pred)
}

/// Return type is `Option<&T>` / `Option<Reference(_)>`.
pub fn method_returns_option_shared_ref(sig: &FunctionSignature) -> bool {
    return_type_is(sig, |ty| {
        matches!(
            ty,
            Type::Option(inner) if matches!(inner.as_ref(), Type::Reference(_))
        )
    })
}

/// Return type is `Option<&mut T>` / `Option<MutableReference(_)>`.
pub fn method_returns_option_mut_ref(sig: &FunctionSignature) -> bool {
    return_type_is(sig, |ty| {
        matches!(
            ty,
            Type::Option(inner) if matches!(inner.as_ref(), Type::MutableReference(_))
        )
    })
}

/// Parsed return type is `Option<&T>`.
pub fn type_is_option_shared_ref(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Option(inner) if matches!(inner.as_ref(), Type::Reference(_))
    )
}

/// Parsed return type is `Option<&mut T>`.
pub fn type_is_option_mut_ref(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Option(inner) if matches!(inner.as_ref(), Type::MutableReference(_))
    )
}

/// Map/set key lookup returning a shared reference inside `Option` (e.g. `HashMap::get`).
pub fn is_map_shared_get_call(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig = lookup_sig(method, receiver_type, registry);
    let Some(sig) = sig else {
        return false;
    };
    if !method_returns_option_shared_ref(sig) {
        return false;
    }
    if method_is_map_key_qualified(method, receiver_type, registry) {
        return true;
    }
    is_collection_key_lookup(sig, 0, receiver_type)
}

/// Name of a method on `receiver_type` that returns `Option<&mut T>`, if any.
/// Prefers the conventional Rust map API name when present; otherwise any match.
pub fn map_option_mut_ref_method_name<'a>(
    receiver_type: Option<&str>,
    registry: &'a SignatureRegistry,
) -> Option<&'a str> {
    let rt = receiver_type?;
    let preferred = format!("{rt}::get_mut");
    if let Some(sig) = registry.get_signature(&preferred) {
        if method_returns_option_mut_ref(sig) {
            return Some("get_mut");
        }
    }
    let prefix = format!("{rt}::");
    for (key, sig) in registry.all_signatures_for_suffix_search() {
        if key.starts_with(&prefix) && method_returns_option_mut_ref(sig) {
            return key.rsplit("::").next();
        }
    }
    None
}

/// Whether the receiver type has a method returning `Option<&mut T>` (map `get_mut` sibling).
pub fn map_has_get_mut_sibling(receiver_type: Option<&str>, registry: &SignatureRegistry) -> bool {
    map_option_mut_ref_method_name(receiver_type, registry).is_some()
}

// ── Map / set type classification (shared with analyzer) ─────────────────

pub fn is_set_type_name(name: &str) -> bool {
    crate::type_classification::is_set_type_name(name)
}

/// Stdlib types whose zero-arg empty ctor clears to Default (writeback take).
pub fn is_stdlib_default_empty_type(name: &str) -> bool {
    crate::type_classification::is_stdlib_default_empty_type(name)
}

pub fn is_set_type(ty: &crate::parser::Type) -> bool {
    match ty {
        crate::parser::Type::Parameterized(base, _) if is_set_type_name(base) => true,
        crate::parser::Type::Reference(inner) | crate::parser::Type::MutableReference(inner) => {
            is_set_type(inner)
        }
        crate::parser::Type::Custom(name) => is_set_type_name(name),
        _ => false,
    }
}

fn is_map_receiver(receiver_type: Option<&str>) -> bool {
    receiver_type.is_some_and(is_map_type_name)
}

// ── Primary query functions ──────────────────────────────────────────────

/// Does this method mutate its receiver (`&mut self`)?
pub fn method_mutates_receiver_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    crate::analyzer::stdlib_method_traits::method_mutates_receiver_qualified(
        method,
        receiver_type,
        registry,
    )
}

pub fn method_call_mutates_receiver(
    method: &str,
    receiver_type_base: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    crate::analyzer::stdlib_method_traits::method_call_mutates_receiver(
        method,
        receiver_type_base,
        registry,
    )
}

/// Is this method definitely read-only (not `&mut self`)?
pub fn is_known_readonly_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    crate::analyzer::stdlib_method_traits::is_known_readonly_qualified(
        method,
        receiver_type,
        registry,
    )
}

/// Is this a known method in the stdlib signatures?
pub fn is_known_stdlib_method_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    lookup_sig(method, receiver_type, registry).is_some()
        || lookup_suffix(method, registry).is_some()
}

/// Clippy-style `len() == 0` → `is_empty()` rewrite is only valid when:
/// 1. the left-hand method is `len` (or `capacity` on types that also expose `is_empty`)
/// 2. the *receiver type* has an `is_empty` signature in the registry
///
/// Do not use suffix/`usize` consensus here — user methods like
/// `SimNetwork::pending_count() -> usize` must not become `is_empty()`.
pub fn method_is_len_like_empty_check(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    if method != "len" && method != "capacity" {
        return false;
    }
    if !method_returns_usize_qualified(method, receiver_type, registry) {
        return false;
    }
    lookup_sig("is_empty", receiver_type, registry).is_some()
}

/// Does this method return `usize`?
pub fn method_returns_usize_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    if let Some(sig) = lookup_sig(method, receiver_type, registry) {
        return return_type_is(sig, is_usize_type);
    }
    if let Some(sig) = lookup_suffix(method, registry) {
        return return_type_is(sig, is_usize_type);
    }
    consensus_return_is_usize(method, registry)
}

fn consensus_return_is_usize(method: &str, registry: &SignatureRegistry) -> bool {
    let pattern = format!("::{method}");
    let mut any = false;
    for (key, sig) in registry.all_signatures_for_suffix_search() {
        if key.ends_with(&pattern) {
            any = true;
            if !return_type_is(sig, is_usize_type) {
                return false;
            }
        }
    }
    any
}

/// Does this method return an iterator?
pub fn method_returns_iterator_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig =
        lookup_sig(method, receiver_type, registry).or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| {
        return_type_is(s, |ty| {
            matches!(ty, Type::Custom(n) if n == "Iterator")
                || matches!(ty, Type::Parameterized(base, _) if base == "Iterator")
        })
    })
}

/// Whether calling this method yields something usable as a `for` iterable.
///
/// Covers explicit `Iterator` / `Iterator<item>` returns and consuming adapters
/// whose meta return is `Self` with owned `self` (e.g. `into_iter`).
pub fn method_returns_iterable_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    if method_returns_iterator_qualified(method, receiver_type, registry) {
        return true;
    }
    let sig =
        lookup_sig(method, receiver_type, registry).or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| {
        s.has_self_receiver
            && matches!(s.param_ownership.first(), Some(OwnershipMode::Owned))
            && return_type_is(s, |ty| matches!(ty, Type::Custom(n) if n == "Self"))
    })
}

/// Whether an `Option` adapter should lower as `.as_ref().method(...)` under `&self`.
///
/// True when `Option::{method}` takes owned `self` (needs `.as_ref()` under borrow).
pub fn option_owned_self_method(method: &str, registry: &SignatureRegistry) -> bool {
    owned_self_method_on(method, "Option", registry)
}

/// True when `Result::{method}` takes owned `self`.
pub fn result_owned_self_method(method: &str, registry: &SignatureRegistry) -> bool {
    owned_self_method_on(method, "Result", registry)
}

fn owned_self_method_on(method: &str, parent: &str, registry: &SignatureRegistry) -> bool {
    let Some(sig) = lookup_sig(method, Some(parent), registry) else {
        return false;
    };
    sig.has_self_receiver && matches!(sig.param_ownership.first(), Some(OwnershipMode::Owned))
}

/// WJ stdlib_meta declares these with borrowed `Option` receivers; Rust's by-value
/// `Option::map` / `and_then` / `or_else` still need the `as_ref` desugar.
pub fn option_adapter_needs_as_ref(method: &str, registry: &SignatureRegistry) -> bool {
    let Some(sig) = lookup_sig(method, Some("Option"), registry) else {
        return false;
    };
    if !sig.has_self_receiver {
        return false;
    }
    let borrowed_option_self = matches!(sig.param_ownership.first(), Some(OwnershipMode::Borrowed))
        && sig.param_types.first().is_some_and(|t| {
            matches!(
                t,
                Type::Reference(inner)
                    if matches!(
                        inner.as_ref(),
                        Type::Custom(n) if n == "Option" || n.starts_with("Option<")
                    ) || matches!(inner.as_ref(), Type::Option(_))
            )
        });
    borrowed_option_self && sig.param_types.get(1).is_some_and(is_closure_type)
}

/// Item type from `Iterator<T>` return metadata (`Parameterized("Iterator", [T])`).
pub fn iterator_item_type_from_sig(sig: &FunctionSignature) -> Option<Type> {
    match sig.return_type.as_ref()? {
        Type::Parameterized(base, params) if base == "Iterator" && params.len() == 1 => {
            Some(params[0].clone())
        }
        _ => None,
    }
}

/// Whether an iterator method yields owned Copy elements (e.g. `chars` → `char`).
/// Driven by `Iterator<item>` return metadata — not method-name lists.
pub fn method_yields_owned_iterator_elements_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    owned_iterator_element_type_qualified(method, receiver_type, registry).is_some_and(|t| {
        crate::codegen::rust::type_analysis_pure::is_copy_type(&t)
            && !matches!(t, Type::Reference(_) | Type::MutableReference(_))
    })
}

/// Element type for by-value string iterators from registry (`chars` → `char`).
pub fn owned_iterator_element_type_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> Option<Type> {
    let sig =
        lookup_sig(method, receiver_type, registry).or_else(|| lookup_suffix(method, registry))?;
    let item = iterator_item_type_from_sig(sig)?;
    // Owned Copy items (char, u8) — not `&str` / `&String` from split/lines.
    if matches!(&item, Type::Reference(_) | Type::MutableReference(_)) {
        return None;
    }
    if crate::codegen::rust::type_analysis_pure::is_copy_type(&item) {
        Some(item)
    } else {
        None
    }
}

/// Whether a method argument is a Rust `Pattern`/`&str` slot (from resolved signature).
pub fn method_arg_is_string_pattern_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
    arg_index: usize,
) -> bool {
    method_arg_expects_rust_str_ref_qualified(method, receiver_type, registry, arg_index)
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
    // Ambiguous suffix (e.g. `clone` on many std types) — require unanimous `Self`.
    consensus_return_is_self(method, registry)
}

/// Is this a storage method that moves a parameter into a collection?
/// Derived from: non-self param is `Owned` (not borrowed) for the stored value.
pub fn method_is_storage_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig =
        lookup_sig(method, receiver_type, registry).or_else(|| lookup_suffix(method, registry));
    if let Some(s) = sig {
        if !s.has_self_receiver || arg_count(s) == 0 {
            return false;
        }
        if s.param_ownership[0] != OwnershipMode::MutBorrowed {
            return false;
        }
        let start = 1; // skip self
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

/// Does the first non-self argument need auto-borrowing (`&str` / `&[T]`)?
pub fn method_auto_borrows_arg_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    method_arg_needs_auto_borrow_at_index(method, receiver_type, registry, 0)
}

/// Does call argument `arg_index` (0 = first arg after receiver) need auto-borrowing?
pub fn method_arg_needs_auto_borrow_at_index(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
    arg_index: usize,
) -> bool {
    let sig =
        lookup_sig(method, receiver_type, registry).or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| {
        let param_idx = s.arg_param_index(arg_index);
        s.param_types.get(param_idx).is_some_and(is_reference_type)
            && matches!(
                s.param_ownership.get(param_idx),
                Some(OwnershipMode::Borrowed)
            )
    })
}

/// Whether a method argument expects `&str` in Rust (from resolved signature).
pub fn method_arg_expects_rust_str_ref_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
    arg_index: usize,
) -> bool {
    let sig =
        lookup_sig(method, receiver_type, registry).or_else(|| lookup_suffix(method, registry));
    sig.and_then(|s| {
        let idx = s.arg_param_index(arg_index);
        s.param_types.get(idx)
    })
    .is_some_and(is_str_reference)
}

/// Whether a method argument expects `&str` in Rust (from a resolved signature).
pub fn method_arg_expects_rust_str_ref_from_sig(sig: &FunctionSignature, arg_index: usize) -> bool {
    let idx = sig.arg_param_index(arg_index);
    sig.param_types.get(idx).is_some_and(is_str_reference)
}

/// Whether a method argument expects a shared borrow of element/key type (`&T`, not `&str`).
pub fn method_arg_expects_borrowed_reference_from_sig(
    sig: &FunctionSignature,
    arg_index: usize,
) -> bool {
    let idx = sig.arg_param_index(arg_index);
    matches!(sig.param_ownership.get(idx), Some(OwnershipMode::Borrowed))
        && sig
            .param_types
            .get(idx)
            .is_some_and(|t| matches!(t, Type::Reference(_)) && !is_str_reference(t))
}

/// Whether a method argument expects a shared borrow (from resolved signature + registry).
pub fn method_arg_expects_borrowed_reference_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
    arg_index: usize,
) -> bool {
    let sig =
        lookup_sig(method, receiver_type, registry).or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| method_arg_expects_borrowed_reference_from_sig(s, arg_index))
}

/// Is this a HashMap/BTreeMap key method whose first arg is a key reference?
pub fn method_is_map_key_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    if !is_map_receiver(receiver_type) {
        // For unknown receiver type, check if method exists on any map type
        if receiver_type.is_some() {
            return false;
        }
        for map_ty in crate::type_classification::MAP_TYPE_NAMES {
            if let Some(sig) = lookup_sig(method, Some(map_ty), registry) {
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
    let sig = lookup_sig(method, receiver_type, registry);
    sig.is_some_and(|s| {
        s.has_self_receiver
            && first_arg_ownership(s) == Some(OwnershipMode::Borrowed)
            && first_arg_type(s).is_some_and(is_reference_type)
    })
}

/// Is this an option accessor that may need `.cloned()` on borrowed receivers?
/// e.g. first, last, unwrap on borrowed Option/Vec
pub fn method_is_option_accessor_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig =
        lookup_sig(method, receiver_type, registry).or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| {
        s.has_self_receiver
            && return_type_is(s, |ty| {
                matches!(ty,
                    Type::Option(inner) if matches!(&**inner, Type::Reference(_))
                ) || matches!(ty, Type::Custom(n) if n == "T")
            })
    })
}

/// Is the first non-self param a capacity/index arg that should be cast to `usize`?
pub fn method_is_capacity_cast_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig =
        lookup_sig(method, receiver_type, registry).or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| first_arg_type(s).is_some_and(is_usize_type))
}

/// Does the first non-self param take a usize index?
pub fn method_is_index_taking_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    method_is_capacity_cast_qualified(method, receiver_type, registry)
}

/// Iterator/adapter protocol: predicate closures receive `&T` (not owned `T`).
pub fn method_predicate_closure_receives_ref(method: &str) -> bool {
    crate::analyzer::stdlib_method_traits::method_predicate_closure_receives_ref(method)
}

/// Does this method take a closure/predicate as its first non-self argument?
pub fn method_is_closure_taking_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    crate::analyzer::stdlib_method_traits::method_is_closure_taking_qualified(
        method,
        receiver_type,
        registry,
    )
}

/// Is this a slice search method (`contains`, `binary_search`) whose first arg
/// needs `&T`?
pub fn method_is_slice_search_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig =
        lookup_sig(method, receiver_type, registry).or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| {
        s.has_self_receiver
            && first_arg_ownership(s) == Some(OwnershipMode::Borrowed)
            && first_arg_type(s).is_some_and(|ty| is_reference_type(ty) && !is_str_reference(ty))
    })
}

/// Is this a string search method (`starts_with`, `ends_with`, `contains`)
/// whose first arg needs `&str`?
pub fn method_is_string_search_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig =
        lookup_sig(method, receiver_type, registry).or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| {
        s.has_self_receiver
            && first_arg_ownership(s) == Some(OwnershipMode::Borrowed)
            && first_arg_type(s).is_some_and(is_str_reference)
    })
}

// ── Convenience wrappers (no receiver type) ──────────────────────────────
// Used at call sites that lack receiver type context.
// Driven by stdlib signature consensus — never by expanding method-name lists.

/// Unqualified fallback: unanimous non-`MutBorrowed` self across stdlib `::{method}` keys.
/// Prefer [`is_known_readonly_qualified`] when receiver type + registry are available.
///
/// Delegates to the analyzer helper so both crates share one consensus implementation.
pub fn is_known_readonly(method: &str) -> bool {
    crate::analyzer::stdlib_method_traits::is_known_readonly(method)
}

/// Unqualified fallback: unanimous `MutBorrowed` self across stdlib `::{method}` keys.
/// Prefer [`method_mutates_receiver_qualified`] when receiver type + registry are available.
///
/// Delegates to the analyzer helper so both crates share one consensus implementation.
pub fn method_mutates_receiver(method: &str) -> bool {
    crate::analyzer::stdlib_method_traits::method_mutates_receiver(method)
}

/// HashMap/BTreeMap key methods — name classification for stdlib trait identity only.
/// Ownership / borrow decisions must use [`method_is_map_key_qualified`],
/// [`method_arg_expects_borrowed_reference_qualified`], or [`is_collection_key_lookup`].
pub fn is_map_key_method(method: &str) -> bool {
    crate::analyzer::stdlib_method_traits::is_map_key_method(method)
}

/// Whether a resolved type name or [`Type`] is a map collection receiver.
pub fn is_map_type_name(name: &str) -> bool {
    crate::type_classification::is_map_type_name(name)
}

pub fn is_map_type(ty: &crate::parser::Type) -> bool {
    match ty {
        crate::parser::Type::Parameterized(base, _) if is_map_type_name(base) => true,
        crate::parser::Type::Reference(inner) | crate::parser::Type::MutableReference(inner) => {
            is_map_type(inner)
        }
        crate::parser::Type::Custom(name) => is_map_type_name(name),
        _ => false,
    }
}

pub fn is_closure_taking_method(method: &str) -> bool {
    method_is_closure_taking_qualified(method, None, SignatureRegistry::stdlib())
}

/// Build `Module::method` / `Type::method` for signature/IR lookup at call sites.
///
/// Only treat an identifier as a runtime module when it was imported (`use std::…`).
/// Never match bare lowercase names against a hardcoded/scanned module list — locals
/// named `json` / `server` must stay instance method calls (`json.find`, `server.serve`).
pub fn module_qualified_method_name(
    receiver_type_name: Option<&str>,
    object: &Expression,
    method: &str,
    is_imported_runtime_std_module: impl Fn(&str) -> bool,
) -> String {
    if let Expression::Identifier { name, .. } = object {
        if is_imported_runtime_std_module(name) {
            return format!("{name}::{method}");
        }
        if name.chars().next().is_some_and(|c| c.is_uppercase()) {
            return format!("{name}::{method}");
        }
    }
    if let Some(tn) = receiver_type_name {
        if tn.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
            return format!("{tn}::{method}");
        }
    }
    method.to_string()
}

/// How a WJ `use std::{module}` should lower to Rust.
pub enum WjStdImportKind {
    /// Native Rust std (`collections` HashMap, `cmp`, `ops`).
    RustStd,
    /// Framework / platform modules with no crate-level `use` line.
    Skip,
    /// `windjammer_runtime::{rust_stem}` (`csv` → `csv_mod`).
    Runtime { rust_stem: String },
}

/// Scan-driven import class for a WJ std module base (`csv`, `http`, `fs::write` → `fs`).
///
/// - Runtime `.rs` stem → `windjammer_runtime::{stem}`
/// - Native rust std overlay (`collections` HashMap, `cmp`, `ops`)
/// - WJ-only `std/*.wj` with no runtime file (dialog, net, …) → skip crate `use`
/// - Otherwise rustc `std::{module}` (`fmt`, `sync`, …)
pub fn classify_wj_std_import(module_base: &str) -> WjStdImportKind {
    let base = module_base.split("::").next().unwrap_or(module_base);
    if matches!(base, "collections" | "cmp" | "ops") {
        return WjStdImportKind::RustStd;
    }
    let registry = SignatureRegistry::stdlib();
    if let Some(stem) = registry.runtime_rust_stem(base) {
        return WjStdImportKind::Runtime {
            rust_stem: stem.to_string(),
        };
    }
    if registry.has_runtime_std_module(base) {
        return WjStdImportKind::Skip;
    }
    WjStdImportKind::RustStd
}

/// Rust `use` line for a scanned runtime stem (`csv` → `csv_mod`, `log` → `log_mod`).
///
/// Globs and subpaths import the stem directly (`log_mod::*`, `log_mod::info`).
/// Bare `use std::log` aliases `_mod`/`_runtime` stems so `log::info` still works.
pub fn format_runtime_std_use(module_name: &str, rust_stem: &str, alias: Option<&str>) -> String {
    let rust_import = format!("windjammer_runtime::{rust_stem}");
    if let Some(alias_name) = alias {
        return format!("use {rust_import} as {alias_name};\n");
    }
    let wj_first = module_name
        .strip_suffix("::*")
        .unwrap_or(module_name)
        .split("::")
        .next()
        .unwrap_or(module_name);
    let rest = module_name.strip_prefix(wj_first).unwrap_or("");
    let renamed = rust_stem.ends_with("_mod") || rust_stem.ends_with("_runtime");
    if renamed && rest.is_empty() {
        let original_name = rust_stem
            .strip_suffix("_mod")
            .or_else(|| rust_stem.strip_suffix("_runtime"))
            .unwrap_or(rust_stem);
        let mut result = format!("use {rust_import} as {original_name};\n");
        let registry = SignatureRegistry::stdlib();
        for ty in registry.runtime_exported_types_for_module(original_name) {
            result.push_str(&format!("use {rust_import}::{ty};\n"));
        }
        return result;
    }
    format!("use {rust_import}{rest};\n")
}

/// Scanned WJ `std::module` name (runtime `.rs` stem + `_mod`/`_runtime` aliases + `std/*.wj`).
pub fn is_runtime_std_module(name: &str) -> bool {
    SignatureRegistry::stdlib().has_runtime_std_module(name)
}

/// Any `::` segment is a scanned runtime std module (`std::db::connect`, `db::connect`).
pub fn callee_path_is_runtime_std(name: &str) -> bool {
    name.split("::").any(is_runtime_std_module)
}

/// `Connection` / `&Connection` — scanned impl type that lives in a runtime module.
pub fn type_is_runtime_asref_receiver(ty: &Type) -> bool {
    let bare = match ty {
        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
        other => other,
    };
    match bare {
        Type::Custom(tn) => runtime_std_module_for_type(tn).is_some(),
        _ => false,
    }
}

/// Module segment of a callee path: `strings`, `strings::substring`, `std::strings::len`.
pub fn runtime_module_segment_from_callee_path(name: &str) -> &str {
    let parts: Vec<&str> = name.split("::").collect();
    match parts.as_slice() {
        ["std", m, ..] => m,
        [m, ..] => m,
        [] => name,
    }
}

/// Stdlib struct types that lower to a `windjammer_runtime` module (scanned `impl Type`).
pub fn runtime_std_module_for_type(type_name: &str) -> Option<&'static str> {
    SignatureRegistry::stdlib().runtime_module_for_type(type_name)
}

/// Scanned runtime Rust signature borrows this arg while the WJ formal is still owned.
///
/// Driven by `stdlib_scanner` / registry `param_ownership` and `emitted_rust_ref_params`
/// (e.g. `AsRef<Path>`, `AsRef<str>`, `&str` on runtime helpers) — never by method or
/// module name lists. Scanner maps AsRef contracts to `Reference(str)` in `param_types`,
/// so the emitted-ref flag is the reliable signal that WJ still passes owned `string`.
pub fn runtime_wj_owned_rust_borrowed_param(
    sig: &crate::analyzer::FunctionSignature,
    arg_index: usize,
) -> bool {
    use crate::analyzer::OwnershipMode;
    use crate::parser::Type;

    let pidx = sig.arg_param_index(arg_index);
    let scanned_borrow = matches!(
        sig.param_ownership.get(pidx),
        Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
    );
    if !scanned_borrow {
        return false;
    }
    if sig
        .emitted_rust_ref_params
        .as_ref()
        .and_then(|flags| flags.get(pidx))
        .copied()
        == Some(true)
    {
        return true;
    }
    sig.formal_param_type(pidx)
        .or_else(|| sig.param_types.get(pidx))
        .is_none_or(|t| !matches!(t, Type::Reference(_) | Type::MutableReference(_)))
}

/// Whether a runtime-std call argument needs `&` at the Rust call site (signature-driven).
pub fn runtime_std_param_needs_auto_borrow(
    signature: Option<&crate::analyzer::FunctionSignature>,
    arg_index: usize,
) -> bool {
    signature.is_some_and(|sig| runtime_wj_owned_rust_borrowed_param(sig, arg_index))
}

fn runtime_std_module_arg_needs_rust_borrow(
    sig: &crate::analyzer::FunctionSignature,
    arg_index: usize,
) -> bool {
    let pidx = sig.arg_param_index(arg_index);
    // Codegen-recorded owned emission wins over stale analyzer Borrowed — user
    // `build_html(name: String)` must peel `&name`, not treat Borrowed as AsRef.
    if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx) {
        return false;
    }
    // `emitted_rust_ref_params[idx] == false` + plain WJ `string` is owned `String` at
    // Rust even when analyzer ownership is still Borrowed (stale multipass). Do not
    // treat that as a runtime AsRef/`&str` contract — peel `&` at the call site.
    if sig
        .emitted_rust_ref_params
        .as_ref()
        .and_then(|flags| flags.get(pidx))
        .copied()
        == Some(false)
        && (crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
            sig, pidx,
        ) || crate::codegen::rust::signature_promotion::bare_formal_is_vec_or_map(sig, pidx)
            || crate::codegen::rust::signature_promotion::bare_formal_is_owned_user_type(
                sig, pidx,
            ))
    {
        return false;
    }
    // Explicit scanned/emitted shared text / AsRef contract.
    if crate::ir::emission_contract::callee_emits_shared_rust_ref_param(sig, pidx)
        && sig.param_types.get(pidx).is_some_and(|t| {
            crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                || crate::codegen::rust::types::is_windjammer_text_type(match t {
                    crate::parser::Type::Reference(inner)
                    | crate::parser::Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                })
        })
    {
        return true;
    }
    // Borrowed + WJ-owned formal (no owned-emission record): keep auto-borrow for
    // cross-crate / body-demoted `&str` contracts (`wdb_circuit::exists`). Owned
    // emission (`Some(false)` above) already returned false.
    runtime_wj_owned_rust_borrowed_param(sig, arg_index)
}

/// Like [`runtime_std_param_needs_auto_borrow`], but when a layered registry shadows the
/// runtime scanner baseline with WJ-owned formals, still honor the baseline borrow contract.
pub fn runtime_std_param_needs_auto_borrow_resolved(
    registry: &crate::analyzer::SignatureRegistry,
    callee_name: &str,
    signature: Option<&crate::analyzer::FunctionSignature>,
    arg_index: usize,
) -> bool {
    // Defining-module codegen refresh beats stale registry/stdlib Borrowed stubs that
    // lack `emitted_rust_ref_params` (multipass owned `Vec` / Custom formals).
    if let Some(sig) = signature {
        let pidx = sig.arg_param_index(arg_index);
        if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx) {
            return false;
        }
    }
    if signature.is_some_and(|sig| runtime_std_module_arg_needs_rust_borrow(sig, arg_index)) {
        return true;
    }
    if let Some(reg_sig) = registry.get_signature(callee_name) {
        let already_checked = signature.is_some_and(|s| std::ptr::eq(s, reg_sig));
        if !already_checked && runtime_std_module_arg_needs_rust_borrow(reg_sig, arg_index) {
            return true;
        }
    }
    if let Some(baseline) = registry.get_fallback_signature(callee_name) {
        if runtime_std_module_arg_needs_rust_borrow(baseline, arg_index) {
            return true;
        }
    }
    // Local codegen registries may not layer `global_fallback`; still consult the
    // shared stdlib/runtime scan. Use the full callee key only — bare `query` would
    // collide across modules (http vs Connection).
    let stdlib = crate::analyzer::SignatureRegistry::stdlib();
    if let Some(baseline) = stdlib.get_signature(callee_name) {
        if runtime_std_module_arg_needs_rust_borrow(baseline, arg_index) {
            return true;
        }
    }
    if let Some(baseline) = stdlib.get_fallback_signature(callee_name) {
        if runtime_std_module_arg_needs_rust_borrow(baseline, arg_index) {
            return true;
        }
    }
    false
}

/// Map/set key lookup: first arg must be borrowed when the receiver is a map/set type.
/// Driven by signature registry ownership/types — not method name lists.
pub fn is_collection_key_lookup(
    sig: &FunctionSignature,
    arg_index: usize,
    receiver_type: Option<&str>,
) -> bool {
    if arg_index != 0 {
        return false;
    }
    let method = sig.name.rsplit("::").next().unwrap_or(&sig.name);
    let registry = SignatureRegistry::stdlib();
    let receiver_base = receiver_type
        .map(|rt| rt.split('<').next().unwrap_or(rt))
        .or_else(|| {
            sig.name.rsplit_once("::").map(|(ty, _)| {
                let bare = ty.rsplit("::").next().unwrap_or(ty);
                bare.split('<').next().unwrap_or(bare)
            })
        });
    if let Some(base) = receiver_base {
        if is_map_type_name(base) || is_set_type_name(base) {
            if callee_arg_expects_reference_param(sig, arg_index) {
                return true;
            }
            return method_is_map_key_qualified(method, receiver_type, registry);
        }
    }
    // Receiver type unknown at codegen (`map` from `Ok(map)`): registry consensus.
    method_is_map_key_qualified(method, receiver_type, registry)
}

/// Extract `Vec` from `Vec::push` for call-site qualification when local type inference failed.
pub fn receiver_type_from_qualified_sig(sig: &FunctionSignature) -> Option<&str> {
    sig.name.rsplit_once("::").map(|(rt, _)| rt)
}

/// Whether the resolved callee expects a shared or mutable reference at `arg_index`.
pub fn callee_arg_expects_reference_param(
    sig: &crate::analyzer::FunctionSignature,
    arg_index: usize,
) -> bool {
    use crate::analyzer::OwnershipMode;
    use crate::parser::Type;

    let pidx = sig.arg_param_index(arg_index);
    if sig
        .param_types
        .get(pidx)
        .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)))
    {
        return true;
    }
    matches!(
        crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
            sig, arg_index,
        ),
        OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
    ) || matches!(
        sig.param_ownership.get(pidx),
        Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
    )
}

/// Resolve the runtime std module for call-site borrow decisions.
pub fn resolve_runtime_std_module<'a>(
    callee_module: &'a str,
    receiver_type: Option<&str>,
) -> &'a str {
    if is_runtime_std_module(callee_module) {
        return callee_module;
    }
    if let Some(tn) = receiver_type {
        if let Some(m) = runtime_std_module_for_type(tn) {
            return m;
        }
    }
    callee_module
}

/// Whether a runtime-std call argument needs `&` inserted at the Rust call site.
///
/// Driven by scanned `param_ownership` / resolved signatures — never by module-name lists.
pub fn runtime_std_call_arg_needs_auto_borrow(
    module: &str,
    _method: &str,
    signature: Option<&crate::analyzer::FunctionSignature>,
    arg_index: usize,
    inferred_type: Option<&crate::parser::Type>,
    arg_expr: &crate::parser::Expression,
    receiver_type: Option<&str>,
) -> bool {
    use crate::analyzer::OwnershipMode;
    use crate::parser::Expression;

    if !matches!(
        arg_expr,
        Expression::Identifier { .. } | Expression::FieldAccess { .. }
    ) {
        return false;
    }

    let module = resolve_runtime_std_module(module, receiver_type);

    if signature.is_some_and(|sig| runtime_wj_owned_rust_borrowed_param(sig, arg_index)) {
        return true;
    }

    let param_idx = signature.map(|s| s.arg_param_index(arg_index));
    if let Some(ownership) =
        param_idx.and_then(|idx| signature.and_then(|s| s.param_ownership.get(idx).copied()))
    {
        if matches!(
            ownership,
            OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
        ) && is_runtime_std_module(module)
        {
            return true;
        }
    }

    // Fail closed without a Borrowed formal: do not guess from module name + text type.
    let _ = inferred_type;
    false
}

/// Signature-driven: string literals must stay bare (`&str`) at this runtime/stdlib formal.
///
/// Does **not** apply to `&String` (`@string_ref`) — those need `&"lit".to_string()`.
pub fn runtime_or_str_ref_formal_skips_literal_owned(
    sig: Option<&crate::analyzer::FunctionSignature>,
    arg_index: usize,
) -> bool {
    let Some(sig) = sig else {
        return false;
    };
    let pidx = sig.arg_param_index(arg_index);
    // Owned `String` formals must never skip `.to_string()`, even with stale Borrowed.
    if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx) {
        return false;
    }
    // `&String` is a shared ref but still needs owned-literal conversion.
    if sig.string_ref_string_formal_for_arg(arg_index)
        || sig
            .param_types
            .get(pidx)
            .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_string_ref)
        || sig
            .formal_param_type(pidx)
            .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_string_ref)
    {
        return false;
    }
    if sig
        .param_types
        .get(pidx)
        .is_some_and(|t| crate::codegen::rust::string_utilities::param_is_owned_string_type(t))
        && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(sig, pidx)
    {
        return false;
    }
    runtime_wj_owned_rust_borrowed_param(sig, arg_index)
        || method_arg_expects_rust_str_ref_from_sig(sig, arg_index)
        || (crate::ir::emission_contract::callee_emits_shared_rust_ref_param(sig, pidx)
            && !sig
                .param_types
                .get(pidx)
                .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_string_ref))
}

#[cfg(test)]
mod pattern_registry_tests {
    use super::*;
    use crate::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
    use crate::parser::Type;

    #[test]
    fn string_find_and_chars_driven_by_stdlib_meta() {
        let reg = SignatureRegistry::new();
        assert!(
            reg.get_signature("String::find").is_some(),
            "stdlib_meta/string.wj.meta must load into SignatureRegistry"
        );
        assert!(method_arg_expects_rust_str_ref_qualified(
            "find",
            Some("String"),
            &reg,
            0
        ));
        assert!(method_returns_iterator_qualified(
            "chars",
            Some("String"),
            &reg
        ));
        assert!(method_yields_owned_iterator_elements_qualified(
            "chars",
            Some("String"),
            &reg
        ));
        assert!(method_returns_iterable_qualified("iter", Some("Vec"), &reg));
    }

    #[test]
    fn method_mutates_receiver_push_via_stdlib_consensus() {
        assert!(
            method_mutates_receiver("push"),
            "Vec::push / String::push are MutBorrowed in stdlib_meta"
        );
        assert!(method_mutates_receiver("insert"));
        assert!(method_mutates_receiver("clear"));
        assert!(method_mutates_receiver("take"));
    }

    #[test]
    fn method_mutates_receiver_readonly_and_unknown_are_false() {
        assert!(!method_mutates_receiver("len"));
        assert!(!method_mutates_receiver("is_empty"));
        assert!(
            !method_mutates_receiver("totally_unknown_method_xyz"),
            "no matching signatures → conservative false"
        );
    }

    #[test]
    fn method_mutates_receiver_replace_ambiguous_without_type() {
        // Option::replace is MutBorrowed; String::replace is Borrowed.
        assert!(
            !method_mutates_receiver("replace"),
            "unqualified consensus must not treat ambiguous replace as mutating"
        );
    }

    #[test]
    fn method_mutates_receiver_qualified_disambiguates_replace() {
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
        assert!(method_mutates_receiver_qualified("push", Some("Vec"), reg));
        assert!(!method_mutates_receiver_qualified("len", Some("Vec"), reg));
    }

    #[test]
    fn hashset_lookup_args_expect_borrowed_element_ref() {
        let reg = SignatureRegistry::stdlib();
        for method in ["contains", "remove"] {
            assert!(
                method_arg_expects_borrowed_reference_qualified(method, Some("HashSet"), reg, 0),
                "HashSet::{method} first arg must be &T"
            );
            assert!(
                method_arg_expects_borrowed_reference_qualified(method, Some("BTreeSet"), reg, 0),
                "BTreeSet::{method} first arg must be &T"
            );
            let sig = reg
                .get_signature(&format!("HashSet::{method}"))
                .unwrap_or_else(|| panic!("missing HashSet::{method} in stdlib_meta"));
            assert!(is_collection_key_lookup(sig, 0, Some("HashSet")));
            assert!(!method_is_index_taking_qualified(
                method,
                Some("HashSet"),
                reg
            ));
        }
    }

    #[test]
    fn vec_remove_index_is_owned_usize_not_borrowed_key() {
        let reg = SignatureRegistry::stdlib();
        assert!(method_is_index_taking_qualified("remove", Some("Vec"), reg));
        assert!(!method_arg_expects_borrowed_reference_qualified(
            "remove",
            Some("Vec"),
            reg,
            0
        ));
        let sig = reg
            .get_signature("Vec::remove")
            .expect("Vec::remove in stdlib_meta");
        assert!(!is_collection_key_lookup(sig, 0, Some("Vec")));
        assert!(first_arg_type(sig).is_some_and(is_usize_type));
    }

    #[test]
    fn hashmap_get_expects_borrowed_key_ref() {
        let reg = SignatureRegistry::stdlib();
        assert!(method_arg_expects_borrowed_reference_qualified(
            "get",
            Some("HashMap"),
            reg,
            0
        ));
        assert!(method_is_map_key_qualified("get", Some("HashMap"), reg));
        assert!(method_arg_expects_borrowed_reference_qualified(
            "contains_key",
            Some("HashMap"),
            reg,
            0
        ));
        let sig = reg
            .get_signature("HashMap::get")
            .expect("HashMap::get in stdlib_meta");
        assert!(is_collection_key_lookup(sig, 0, Some("HashMap")));
        assert!(method_returns_option_shared_ref(sig));
        assert!(is_map_shared_get_call("get", Some("HashMap"), reg));
        assert!(map_has_get_mut_sibling(Some("HashMap"), reg));
        assert!(!is_map_shared_get_call("get", Some("String"), reg));
    }

    #[test]
    fn remove_suffix_conflicts_on_first_arg_ownership() {
        let reg = SignatureRegistry::stdlib();
        assert!(
            reg.suffix_has_conflicting_first_arg_ownership("remove", 1),
            "Vec::remove(Owned) vs HashMap::remove(Borrowed) must conflict"
        );
        assert!(
            !reg.suffix_has_conflicting_first_arg_ownership("contains_key", 1),
            "map contains_key entries should agree on Borrowed key"
        );
    }

    #[test]
    fn user_get_named_method_is_not_map_shared_get() {
        let reg = SignatureRegistry::stdlib();
        assert!(!is_map_shared_get_call("get", Some("Holder"), reg));
        assert!(!map_has_get_mut_sibling(Some("Holder"), reg));
    }

    #[test]
    fn runtime_std_glob_imports_renamed_stem_items() {
        assert_eq!(
            format_runtime_std_use("log::*", "log_mod", None),
            "use windjammer_runtime::log_mod::*;\n"
        );
        assert_eq!(
            format_runtime_std_use("csv::*", "csv_mod", None),
            "use windjammer_runtime::csv_mod::*;\n"
        );
        assert_eq!(
            format_runtime_std_use("http::*", "http", None),
            "use windjammer_runtime::http::*;\n"
        );
        assert_eq!(
            format_runtime_std_use("log::info", "log_mod", None),
            "use windjammer_runtime::log_mod::info;\n"
        );
        let bare = format_runtime_std_use("log", "log_mod", None);
        assert!(
            bare.contains("use windjammer_runtime::log_mod as log;"),
            "bare std::log must still alias the module: {bare}"
        );
        assert!(!bare.contains("log_mod::*;"));
    }

    #[test]
    fn runtime_borrow_is_signature_driven_not_module_name() {
        let mut sig = FunctionSignature::default();
        sig.name = "wdb_circuit::exists".into();
        sig.param_types = vec![Type::String];
        sig.formal_param_types = vec![Type::String];
        sig.param_ownership = vec![OwnershipMode::Borrowed];
        // Bare WJ string + analyzer Borrowed → Rust `&str` at the call site.
        assert!(runtime_wj_owned_rust_borrowed_param(&sig, 0));
        let mut reg = SignatureRegistry::empty();
        reg.add_function(sig.name.clone(), sig.clone());
        assert!(
            runtime_std_param_needs_auto_borrow_resolved(
                &reg,
                "wdb_circuit::exists",
                Some(&sig),
                0
            ),
            "Borrowed WJ-owned formal must auto-borrow even when the qualifier is not a runtime std module"
        );
        let mut owned = sig.clone();
        owned.param_ownership = vec![OwnershipMode::Owned];
        assert!(!runtime_wj_owned_rust_borrowed_param(&owned, 0));
        let mut owned_reg = SignatureRegistry::empty();
        owned_reg.add_function(owned.name.clone(), owned.clone());
        assert!(!runtime_std_param_needs_auto_borrow_resolved(
            &owned_reg,
            "wdb_circuit::exists",
            Some(&owned),
            0
        ));
    }

    #[test]
    fn owned_string_emission_does_not_force_runtime_borrow() {
        use crate::parser::Type;
        let mut sig = FunctionSignature::default();
        sig.name = "build_html".into();
        sig.param_types = vec![Type::String, Type::String];
        sig.formal_param_types = vec![Type::String, Type::String];
        // Stale analyzer Borrowed must not beat codegen-owned `String` emission.
        sig.param_ownership = vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed];
        sig.emitted_rust_ref_params = Some(vec![false, false]);
        let mut reg = SignatureRegistry::empty();
        reg.add_function(sig.name.clone(), sig.clone());
        assert!(
            !runtime_std_param_needs_auto_borrow_resolved(&reg, "build_html", Some(&sig), 0),
            "owned String emission must peel `&name`, not keep AsRef-style borrow"
        );
    }
}
