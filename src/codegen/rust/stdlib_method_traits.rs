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
    // Windjammer `Map<K,V>` lowers to Rust `HashMap` — stdlib_meta keys are HashMap::*.
    if matches!(leaf, "Map") {
        push("HashMap");
    }
    out
}

/// Attempt a type-qualified signature lookup, trying multiple receiver type
/// representations (e.g. `Vec`, `Vec<T>`, bare generic base).
fn lookup_sig<'a>(
    method: &str,
    receiver_type: Option<&str>,
    registry: &'a SignatureRegistry,
) -> Option<&'a FunctionSignature> {
    if let Some(ty) = receiver_type {
        for candidate in stdlib_receiver_lookup_candidates(ty) {
            let qualified = format!("{candidate}::{method}");
            if let Some(sig) = registry.get_signature(&qualified) {
                return Some(sig);
            }
        }
    }
    None
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

// ── Map type constants ───────────────────────────────────────────────────

const MAP_TYPES: &[&str] = &["HashMap", "BTreeMap", "Map", "IndexMap"];
const SET_TYPES: &[&str] = &["HashSet", "BTreeSet"];

pub fn is_set_type_name(name: &str) -> bool {
    let base = name.split('<').next().unwrap_or(name);
    let short = base.rsplit("::").next().unwrap_or(base);
    SET_TYPES.contains(&short)
}

/// Stdlib types whose zero-arg empty ctor clears to Default (writeback take).
pub fn is_stdlib_default_empty_type(name: &str) -> bool {
    let base = name.split('<').next().unwrap_or(name);
    let short = base.rsplit("::").next().unwrap_or(base);
    is_map_type_name(short)
        || is_set_type_name(short)
        || matches!(short, "Vec" | "VecDeque" | "String")
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

fn sig_mutates_receiver(sig: &FunctionSignature) -> bool {
    sig.has_self_receiver
        && matches!(
            sig.param_ownership.first(),
            Some(OwnershipMode::MutBorrowed)
        )
}

/// True when the receiver is not `&mut self` (`Owned` and `Borrowed` both count).
/// Mirror of `analyzer::stdlib_method_traits::sig_readonly_receiver`.
fn sig_readonly_receiver(sig: &FunctionSignature) -> bool {
    sig.has_self_receiver
        && sig
            .param_ownership
            .first()
            .is_some_and(|o| *o != OwnershipMode::MutBorrowed)
}

/// When `::{method}` is registered on many types, true only if every *instance*
/// method match takes `&mut self` (`MutBorrowed`). Free functions that share the
/// suffix (e.g. `collections::take`) are ignored. Ambiguous method names
/// (e.g. `Option::replace` vs `String::replace`) correctly return false — use
/// the qualified API instead.
fn consensus_mutates_receiver(method: &str, registry: &SignatureRegistry) -> bool {
    let pattern = format!("::{method}");
    let mut any = false;
    for (key, sig) in registry.all_signatures_for_suffix_search() {
        if key.ends_with(&pattern) {
            // Free functions / associated fns share the suffix but have no self.
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
/// method match does not take `&mut self`. Free functions that share the suffix
/// are ignored. Mirror of `analyzer::stdlib_method_traits::consensus_readonly_receiver`.
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

/// Does this method mutate its receiver (`&mut self`)?
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
    consensus_mutates_receiver(method, registry)
}

/// Is this method definitely read-only (not `&mut self`)?
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
    consensus_readonly_receiver(method, registry)
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
        for map_ty in MAP_TYPES {
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
    matches!(
        method,
        "get" | "get_mut" | "contains_key" | "remove" | "get_key_value"
    )
}

/// Whether a resolved type name or [`Type`] is a map collection receiver.
pub fn is_map_type_name(name: &str) -> bool {
    let base = name.split('<').next().unwrap_or(name);
    let short = base.rsplit("::").next().unwrap_or(base);
    matches!(short, "HashMap" | "BTreeMap" | "Map" | "IndexMap")
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

/// Module names from `use std::…` that map to `windjammer_runtime::*` imports.
/// Build `Module::method` for signature/IR lookup at module-style call sites.
pub fn module_qualified_method_name(
    receiver_type_name: Option<&str>,
    object: &Expression,
    method: &str,
    is_imported_runtime_std_module: impl Fn(&str) -> bool,
) -> String {
    if let Some(tn) = receiver_type_name {
        if tn.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
            return format!("{tn}::{method}");
        }
    }
    if let Expression::Identifier { name, .. } = object {
        if is_imported_runtime_std_module(name)
            || is_runtime_std_module(name)
            || name.chars().next().is_some_and(|c| c.is_uppercase())
        {
            return format!("{name}::{method}");
        }
    }
    method.to_string()
}

pub fn is_runtime_std_module(name: &str) -> bool {
    matches!(
        name,
        "http"
            | "server"
            | "strings"
            | "json"
            | "jwt"
            | "time"
            | "math"
            | "random"
            | "mime"
            | "subprocess"
            | "async_runtime"
            | "async"
            | "cli"
            | "crypto"
            | "csv"
            | "db"
            | "regex"
            | "testing"
            | "game"
            | "env"
            | "fs"
    )
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

/// Stdlib struct types that lower to a `windjammer_runtime` module (receiver type → module).
pub fn runtime_std_module_for_type(type_name: &str) -> Option<&'static str> {
    let base = type_name.rsplit("::").next().unwrap_or(type_name);
    match base {
        "Connection" | "Row" | "DatabaseType" => Some("db"),
        _ => None,
    }
}

/// Scanned runtime Rust signature borrows this arg while the WJ formal is still owned.
///
/// Driven by `stdlib_scanner` / registry `param_ownership` (e.g. `subprocess::spawn`'s
/// `&str`, `json::get`'s `&Value`) — never by method or module name lists.
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
    // Signature-driven only: scanned Borrowed + WJ-owned formal. Do not gate on
    // module-name lists (`is_runtime_std_module`) — that misses FFI wrappers and
    // crate-prefixed runtime paths that still register Borrowed formals.
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
    if signature
        .is_some_and(|sig| runtime_std_module_arg_needs_rust_borrow(sig, arg_index))
    {
        return true;
    }
    if let Some(reg_sig) = registry.get_signature(callee_name) {
        let already_checked = signature.is_some_and(|s| std::ptr::eq(s, reg_sig));
        if !already_checked
            && runtime_std_module_arg_needs_rust_borrow(reg_sig, arg_index)
        {
            return true;
        }
    }
    if let Some(baseline) = registry.get_fallback_signature(callee_name) {
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
    let Some(rt) = receiver_type else {
        return false;
    };
    let base = rt.split('<').next().unwrap_or(rt);
    if !is_map_type_name(base) && !is_set_type_name(base) {
        return false;
    }
    callee_arg_expects_reference_param(sig, arg_index)
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
    if sig
        .param_types
        .get(pidx)
        .is_some_and(|t| crate::codegen::rust::string_utilities::param_is_owned_string_type(t))
        && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, pidx)
    {
        return false;
    }
    runtime_wj_owned_rust_borrowed_param(sig, arg_index)
        || method_arg_expects_rust_str_ref_from_sig(sig, arg_index)
        || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, pidx)
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
    fn consensus_mutates_receiver_empty_registry_is_false() {
        let empty = SignatureRegistry::empty();
        assert!(!consensus_mutates_receiver("push", &empty));
        assert!(!method_mutates_receiver_qualified(
            "push",
            Some("Vec"),
            &empty
        ));
    }

    #[test]
    fn consensus_mutates_receiver_mixed_ownership_is_false() {
        let mut reg = SignatureRegistry::empty();
        let mut a = FunctionSignature::default();
        a.name = "A::flip".into();
        a.param_types = vec![Type::Custom("A".into())];
        a.param_ownership = vec![OwnershipMode::MutBorrowed];
        a.has_self_receiver = true;
        reg.add_function("A::flip".into(), a);

        let mut b = FunctionSignature::default();
        b.name = "B::flip".into();
        b.param_types = vec![Type::Custom("B".into())];
        b.param_ownership = vec![OwnershipMode::Borrowed];
        b.has_self_receiver = true;
        reg.add_function("B::flip".into(), b);

        assert!(!consensus_mutates_receiver("flip", &reg));
        assert!(method_mutates_receiver_qualified("flip", Some("A"), &reg));
        assert!(!method_mutates_receiver_qualified("flip", Some("B"), &reg));
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
        assert!(is_known_readonly("len"));
        assert!(!is_known_readonly("push"));
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
}
