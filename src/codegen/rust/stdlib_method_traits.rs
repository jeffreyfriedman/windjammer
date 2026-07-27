//! Signature-driven method behavior queries.
//!
//! Every query first attempts a type-qualified lookup in `SignatureRegistry`
//! (e.g. `Vec::push`), deriving the answer from `FunctionSignature` fields.
//! For non-derivable behaviors (strip-redundant, desugar, ambiguity guards),
//! small const tables live in `rust_stdlib_annotations`.

use crate::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use crate::parser::{Expression, Type};

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
        let short = base.rsplit("::").next().unwrap_or(base);
        for candidate in [base, short, ty] {
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
const SET_TYPES: &[&str] = &["HashSet", "BTreeSet"];

pub fn is_set_type_name(name: &str) -> bool {
    let base = name.split('<').next().unwrap_or(name);
    let short = base.rsplit("::").next().unwrap_or(base);
    SET_TYPES.contains(&short)
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
    let sig =
        lookup_sig(method, receiver_type, registry).or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| return_type_is(s, is_usize_type))
}

/// Does this method return an iterator?
pub fn method_returns_iterator_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig =
        lookup_sig(method, receiver_type, registry).or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| return_type_is(s, |ty| matches!(ty, Type::Custom(n) if n == "Iterator")))
}

/// Is this method type-preserving (return type == `Self`)?
/// e.g. clone, to_owned, to_vec, into_iter
pub fn method_is_type_preserving_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig =
        lookup_sig(method, receiver_type, registry).or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| return_type_is(s, |ty| matches!(ty, Type::Custom(n) if n == "Self")))
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
        s.param_types
            .get(param_idx)
            .is_some_and(is_reference_type)
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
pub fn method_arg_expects_rust_str_ref_from_sig(
    sig: &FunctionSignature,
    arg_index: usize,
) -> bool {
    let idx = sig.arg_param_index(arg_index);
    sig.param_types
        .get(idx)
        .is_some_and(is_str_reference)
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

/// Does this method take a closure/predicate as its first non-self argument?
pub fn method_is_closure_taking_qualified(
    method: &str,
    receiver_type: Option<&str>,
    registry: &SignatureRegistry,
) -> bool {
    let sig =
        lookup_sig(method, receiver_type, registry).or_else(|| lookup_suffix(method, registry));
    sig.is_some_and(|s| first_arg_type(s).is_some_and(is_closure_type))
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

pub fn method_mutates_receiver(method: &str) -> bool {
    matches!(
        method,
        "push"
            | "pop"
            | "insert"
            | "remove"
            | "clear"
            | "append"
            | "extend"
            | "drain"
            | "truncate"
            | "resize"
            | "retain"
            | "sort"
            | "sort_by"
            | "sort_by_key"
            | "sort_unstable"
            | "sort_unstable_by"
            | "dedup"
            | "reverse"
            | "swap"
            | "swap_remove"
            | "reserve"
            | "shrink_to_fit"
            | "split_off"
            | "fill"
            | "set"
            | "rotate_left"
            | "rotate_right"
            | "set_len"
            | "push_str"
            | "push_front"
            | "push_back"
            | "pop_front"
            | "pop_back"
            | "make_ascii_lowercase"
            | "make_ascii_uppercase"
            | "add"
            | "take"
            | "replace"
            | "get_or_insert"
            | "get_or_insert_with"
            | "entry"
            | "get_mut"
            | "iter_mut"
            | "values_mut"
    )
}

/// String methods whose pattern/delimiter args lower to `&str` in Rust (not owned `String`).
pub fn is_string_pattern_method(method: &str) -> bool {
    matches!(
        method,
        "replace"
            | "replacen"
            | "split"
            | "splitn"
            | "rsplit"
            | "split_whitespace"
            | "contains"
            | "starts_with"
            | "ends_with"
            | "find"
            | "rfind"
            | "match_indices"
            | "strip_prefix"
            | "strip_suffix"
            | "trim"
            | "trim_start"
            | "trim_end"
    )
}

pub fn method_returns_iterator(method: &str) -> bool {
    matches!(
        method,
        "iter"
            | "iter_mut"
            | "into_iter"
            | "keys"
            | "values"
            | "values_mut"
            | "drain"
            | "lines"
            | "chars"
            | "bytes"
            | "split"
            | "split_whitespace"
            | "enumerate"
            | "windows"
            | "chunks"
            | "match_indices"
            | "rsplit"
            | "splitn"
    )
}

pub fn is_map_key_method(method: &str) -> bool {
    matches!(
        method,
        "get" | "get_mut" | "contains_key" | "remove" | "get_key_value"
    )
}

/// HashSet/BTreeSet lookup — first arg is always borrowed.
pub fn is_set_lookup_method(method: &str) -> bool {
    matches!(method, "contains" | "remove")
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

pub fn is_index_taking_method(method: &str) -> bool {
    matches!(
        method,
        "insert" | "remove" | "swap" | "swap_remove" | "drain" | "split_off"
    )
}

pub fn is_closure_taking_method(method: &str) -> bool {
    matches!(
        method,
        "filter"
            | "any"
            | "all"
            | "find"
            | "find_map"
            | "position"
            | "take_while"
            | "skip_while"
            | "map_while"
            | "partition"
            | "rposition"
            | "retain"
            | "sort_by"
            | "sort_by_key"
            | "sort_unstable_by"
    )
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
        return format!("{tn}::{method}");
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
        "strings"
            | "json"
            | "jwt"
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
    matches!(
        module,
        "strings" | "json" | "jwt" | "regex" | "csv" | "mime" | "http" | "env" | "db"
    )
}

/// Stdlib struct types that lower to a `windjammer_runtime` module (receiver type → module).
pub fn runtime_std_module_for_type(type_name: &str) -> Option<&'static str> {
    match type_name {
        "Connection" | "Row" | "DatabaseType" => Some("db"),
        _ => None,
    }
}

/// Whether a method call receiver uses an AsRef<str> runtime std module.
pub fn receiver_uses_asref_str_runtime_module(
    runtime_module: Option<&str>,
    receiver_type: Option<&str>,
    is_imported_runtime_std_module: impl Fn(&str) -> bool,
) -> bool {
    if runtime_module.is_some_and(runtime_std_module_uses_asref_str) {
        return true;
    }
    if let Some(tn) = receiver_type {
        if let Some(m) = runtime_std_module_for_type(tn) {
            return runtime_std_module_uses_asref_str(m);
        }
        if is_imported_runtime_std_module(tn) {
            return runtime_std_module_uses_asref_str(tn);
        }
    }
    false
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
    callee_name: &str,
    sig: &crate::analyzer::FunctionSignature,
    arg_index: usize,
) -> bool {
    if !runtime_wj_owned_rust_borrowed_param(sig, arg_index) {
        return false;
    }
    let module = callee_name.split("::").next().unwrap_or("");
    is_runtime_std_module(module)
}

/// Like [`runtime_std_param_needs_auto_borrow`], but when a layered registry shadows the
/// runtime scanner baseline with WJ-owned formals, still honor the baseline borrow contract.
pub fn runtime_std_param_needs_auto_borrow_resolved(
    registry: &crate::analyzer::SignatureRegistry,
    callee_name: &str,
    signature: Option<&crate::analyzer::FunctionSignature>,
    arg_index: usize,
) -> bool {
    if signature.is_some_and(|sig| runtime_std_module_arg_needs_rust_borrow(callee_name, sig, arg_index))
    {
        return true;
    }
    if let Some(reg_sig) = registry.get_signature(callee_name) {
        let already_checked = signature.is_some_and(|s| std::ptr::eq(s, reg_sig));
        if !already_checked
            && runtime_std_module_arg_needs_rust_borrow(callee_name, reg_sig, arg_index)
        {
            return true;
        }
    }
    if let Some(baseline) = registry.get_fallback_signature(callee_name) {
        if runtime_std_module_arg_needs_rust_borrow(callee_name, baseline, arg_index) {
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
    if sig.param_types.get(pidx).is_some_and(|t| {
        matches!(t, Type::Reference(_) | Type::MutableReference(_))
    }) {
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
    if is_runtime_std_module(callee_module) || runtime_std_module_uses_asref_str(callee_module) {
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
/// Uses param types when available; falls back to scanned `param_ownership` (runtime
/// scanner often has empty `param_types` but correct borrow hints from Rust signatures).
pub fn runtime_std_call_arg_needs_auto_borrow(
    module: &str,
    method: &str,
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
    let param_idx = signature.map(|s| s.arg_param_index(arg_index));
    let sig_type = param_idx.and_then(|idx| {
        signature.and_then(|s| {
            s.formal_param_type(idx)
                .or_else(|| s.param_types.get(idx))
        })
    });

    if signature.is_some_and(|sig| runtime_wj_owned_rust_borrowed_param(sig, arg_index)) {
        return true;
    }

    if let Some(ty) = sig_type.or(inferred_type) {
        if runtime_std_module_uses_asref_str(module)
            && (crate::codegen::rust::string_utilities::param_is_owned_string_type(ty)
                || crate::codegen::rust::types::is_windjammer_text_type(ty))
        {
            return true;
        }
    }

    if let Some(ownership) = param_idx.and_then(|idx| {
        signature.and_then(|s| s.param_ownership.get(idx).copied())
    }) {
        if matches!(ownership, OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
            && is_runtime_std_module(module)
        {
            return true;
        }
    }

    runtime_std_module_uses_asref_str(module)
        && inferred_type.is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
}
