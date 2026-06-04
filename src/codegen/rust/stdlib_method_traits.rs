//! Signature-driven method behavior queries.
//!
//! Every query first attempts a type-qualified lookup in `SignatureRegistry`
//! (e.g. `Vec::push`), deriving the answer from `FunctionSignature` fields.
//! For non-derivable behaviors (strip-redundant, desugar, ambiguity guards),
//! small const tables live in `rust_stdlib_annotations`.

use crate::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use crate::parser::Type;

// ── Helpers ──────────────────────────────────────────────────────────────

/// Attempt a type-qualified signature lookup, trying multiple receiver type
/// representations (e.g. `Vec`, `Vec<T>`, bare generic base).
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

/// Suffix lookup fallback: finds signatures registered as `Type::method`
/// when the receiver type is unknown. Only returns a result when there is
/// exactly one candidate — if multiple types define the same method name,
/// we cannot disambiguate and must use default behavior.
fn lookup_suffix<'a>(
    method: &str,
    registry: &'a SignatureRegistry,
) -> Option<&'a FunctionSignature> {
    let pattern = format!("::{}", method);
    let mut matches = registry
        .signatures
        .iter()
        .filter(|(key, _)| key.ends_with(&pattern));
    let first = matches.next();
    if matches.next().is_some() {
        return None; // ambiguous
    }
    first.map(|(_, sig)| sig)
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

// ── Map type constants ───────────────────────────────────────────────────

const MAP_TYPES: &[&str] = &["HashMap", "BTreeMap", "Map", "IndexMap"];

fn is_map_receiver(receiver_type: Option<&str>) -> bool {
    receiver_type.is_some_and(|ty| {
        let base = ty.split('<').next().unwrap_or(ty);
        MAP_TYPES.contains(&base)
    })
}

// ── Primary query functions ──────────────────────────────────────────────

/// Does this method mutate its receiver (`&mut self`)?
pub fn method_mutates_receiver_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    if let Some(sig) = lookup_sig(method, receiver_type, registry) {
        if sig.has_self_receiver && !sig.param_ownership.is_empty() {
            return sig.param_ownership[0] == OwnershipMode::MutBorrowed;
        }
    }
    if let Some(sig) = lookup_suffix(method, registry) {
        if sig.has_self_receiver && !sig.param_ownership.is_empty() {
            return sig.param_ownership[0] == OwnershipMode::MutBorrowed;
        }
    }
    false
}

/// Is this method definitely read-only (`&self`)?
pub fn is_known_readonly_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    if let Some(sig) = lookup_sig(method, receiver_type, registry) {
        if sig.has_self_receiver && !sig.param_ownership.is_empty() {
            return sig.param_ownership[0] != OwnershipMode::MutBorrowed;
        }
    }
    if let Some(sig) = lookup_suffix(method, registry) {
        if sig.has_self_receiver && !sig.param_ownership.is_empty() {
            return sig.param_ownership[0] != OwnershipMode::MutBorrowed;
        }
    }
    false
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

/// Does this method return `usize`?
pub fn method_returns_usize_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig = lookup_sig(method, receiver_type, registry)
        .or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| return_type_is(s, |ty| is_usize_type(ty)))
}

/// Does this method return an iterator?
pub fn method_returns_iterator_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig = lookup_sig(method, receiver_type, registry)
        .or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| {
        return_type_is(s, |ty| matches!(ty, Type::Custom(n) if n == "Iterator"))
    })
}

/// Is this method type-preserving (return type == `Self`)?
/// e.g. clone, to_owned, to_vec, into_iter
pub fn method_is_type_preserving_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig = lookup_sig(method, receiver_type, registry)
        .or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| {
        return_type_is(s, |ty| matches!(ty, Type::Custom(n) if n == "Self"))
    })
}

/// Is this a storage method that moves a parameter into a collection?
/// Derived from: non-self param is `Owned` (not borrowed) for the stored value.
pub fn method_is_storage_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig = lookup_sig(method, receiver_type, registry)
        .or_else(|| lookup_suffix(method, registry));
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
    let sig = lookup_sig(method, receiver_type, registry)
        .or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| {
        first_arg_type(s).is_some_and(|ty| is_reference_type(ty))
            && first_arg_ownership(s) == Some(OwnershipMode::Borrowed)
    })
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
                if sig.has_self_receiver && first_arg_ownership(sig) == Some(OwnershipMode::Borrowed) {
                    if first_arg_type(sig).is_some_and(|ty| is_reference_type(ty)) {
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
            && first_arg_type(s).is_some_and(|ty| is_reference_type(ty))
    })
}

/// Is this an option accessor that may need `.cloned()` on borrowed receivers?
/// e.g. first, last, unwrap on borrowed Option/Vec
pub fn method_is_option_accessor_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig = lookup_sig(method, receiver_type, registry)
        .or_else(|| lookup_suffix(method, registry));
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
    let sig = lookup_sig(method, receiver_type, registry)
        .or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| first_arg_type(s).is_some_and(|ty| is_usize_type(ty)))
}

/// Does the first non-self param take a usize index?
pub fn method_is_index_taking_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    method_is_capacity_cast_qualified(method, receiver_type, registry)
}

/// Does this method take a closure/predicate as its first non-self argument?
pub fn method_is_closure_taking_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig = lookup_sig(method, receiver_type, registry)
        .or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| first_arg_type(s).is_some_and(|ty| is_closure_type(ty)))
}

/// Is this a slice search method (`contains`, `binary_search`) whose first arg
/// needs `&T`?
pub fn method_is_slice_search_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig = lookup_sig(method, receiver_type, registry)
        .or_else(|| lookup_suffix(method, registry));
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
    let sig = lookup_sig(method, receiver_type, registry)
        .or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| {
        s.has_self_receiver
            && first_arg_ownership(s) == Some(OwnershipMode::Borrowed)
            && first_arg_type(s).is_some_and(|ty| is_str_reference(ty))
    })
}

// ── Convenience wrappers (no receiver type) ──────────────────────────────
// Used at call sites that lack receiver type context.

pub fn method_mutates_receiver(method: &str) -> bool {
    matches!(
        method,
        "push" | "pop" | "insert" | "remove" | "clear" | "append" | "extend"
            | "drain" | "truncate" | "resize" | "retain" | "sort" | "sort_by"
            | "sort_by_key" | "sort_unstable" | "sort_unstable_by" | "dedup"
            | "reverse" | "swap" | "swap_remove" | "reserve" | "shrink_to_fit"
            | "split_off" | "fill" | "set" | "rotate_left" | "rotate_right"
            | "set_len" | "push_str" | "push_front" | "push_back" | "pop_front"
            | "pop_back" | "make_ascii_lowercase" | "make_ascii_uppercase" | "add"
            | "take" | "replace" | "get_or_insert" | "get_or_insert_with" | "entry"
            | "get_mut" | "iter_mut" | "values_mut"
    )
}

pub fn method_returns_iterator(method: &str) -> bool {
    matches!(
        method,
        "iter" | "iter_mut" | "into_iter" | "keys" | "values" | "values_mut"
            | "drain" | "lines" | "chars" | "bytes" | "split" | "split_whitespace"
            | "enumerate" | "windows" | "chunks" | "match_indices" | "rsplit"
            | "splitn"
    )
}

pub fn is_map_key_method(method: &str) -> bool {
    matches!(method, "get" | "get_mut" | "contains_key" | "remove" | "get_key_value")
}

pub fn is_index_taking_method(method: &str) -> bool {
    matches!(
        method,
        "insert" | "remove" | "swap" | "swap_remove" | "drain" | "split_off"
    )
}

pub fn is_closure_taking_method(method: &str) -> bool {
    matches!(
        method,
        "filter" | "any" | "all" | "find" | "find_map" | "position"
            | "take_while" | "skip_while" | "map_while" | "partition"
            | "rposition" | "retain" | "sort_by" | "sort_by_key"
            | "sort_unstable_by"
    )
}

/// Module names from `use std::…` that map to `windjammer_runtime::*` imports.
pub fn is_runtime_std_module(name: &str) -> bool {
    matches!(
        name,
        "strings"
            | "json"
            | "time"
            | "math"
            | "random"
            | "http"
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
    )
}

/// Runtime std modules whose Rust implementations take `AsRef<str>` for Windjammer `string` params.
pub fn runtime_std_module_uses_asref_str(module: &str) -> bool {
    matches!(module, "strings" | "json" | "regex" | "csv" | "mime" | "http" | "env")
}
