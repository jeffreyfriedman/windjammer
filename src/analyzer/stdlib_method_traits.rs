//! Stdlib method behavior queries for the analyzer.
//!
//! SignatureRegistry-backed qualified lookups when a receiver type and registry
//! are available; inline name-based fallbacks otherwise (legacy method_registry parity).

use crate::parser::Type;

use super::{FunctionSignature, OwnershipMode, SignatureRegistry};

// ── Inline fallback tables (legacy method_registry parity) ───────────────

const MUTATES_RECEIVER: &[&str] = &[
    "push",
    "pop",
    "insert",
    "remove",
    "clear",
    "append",
    "extend",
    "drain",
    "truncate",
    "resize",
    "retain",
    "sort",
    "sort_by",
    "sort_by_key",
    "sort_unstable",
    "sort_unstable_by",
    "dedup",
    "reverse",
    "swap",
    "swap_remove",
    "reserve",
    "shrink_to_fit",
    "split_off",
    "fill",
    "set",
    "rotate_left",
    "rotate_right",
    "set_len",
    "push_str",
    "push_front",
    "push_back",
    "pop_front",
    "pop_back",
    "make_ascii_lowercase",
    "make_ascii_uppercase",
    "add",
    "take",
    "replace",
    "get_or_insert",
    "get_or_insert_with",
    "entry",
    "get_mut",
    "iter_mut",
    "values_mut",
];

const KNOWN_READONLY: &[&str] = &[
    "contains_key",
    "get",
    "get_key_value",
    "len",
    "is_empty",
    "contains",
    "first",
    "last",
    "capacity",
    "keys",
    "values",
    "binary_search",
    "to_le_bytes",
    "to_be_bytes",
    "from_le_bytes",
    "from_be_bytes",
    "iter",
    "into_iter",
    "windows",
    "chunks",
    "enumerate",
    "lines",
    "chars",
    "bytes",
    "split",
    "split_whitespace",
    "clone",
    "to_string",
    "to_owned",
    "to_vec",
    "as_str",
    "as_ref",
    "as_slice",
    "as_bytes",
    "as_deref",
    "as_ptr",
    "as_mut_ptr",
    "trim",
    "starts_with",
    "ends_with",
    "to_lowercase",
    "to_uppercase",
    "is_ascii",
    "substring",
    "splitn",
    "rsplit",
    "repeat",
    "replacen",
    "rfind",
    "match_indices",
    "trim_start",
    "trim_end",
    "strip_prefix",
    "strip_suffix",
    "to_ascii_lowercase",
    "to_ascii_uppercase",
    "slice",
    "reversed",
    "extend_from_slice",
    "count",
    "with_capacity",
    "abs",
    "ceil",
    "floor",
    "round",
    "sqrt",
    "cbrt",
    "powi",
    "powf",
    "sin",
    "cos",
    "tan",
    "asin",
    "acos",
    "atan",
    "atan2",
    "sinh",
    "cosh",
    "tanh",
    "asinh",
    "acosh",
    "atanh",
    "log",
    "log2",
    "log10",
    "ln",
    "exp",
    "exp2",
    "min",
    "max",
    "clamp",
    "signum",
    "copysign",
    "fract",
    "recip",
    "to_radians",
    "to_degrees",
    "is_nan",
    "is_infinite",
    "is_finite",
    "is_normal",
    "is_sign_positive",
    "is_sign_negative",
    "leading_zeros",
    "trailing_zeros",
    "count_ones",
    "count_zeros",
    "wrapping_add",
    "wrapping_sub",
    "wrapping_mul",
    "saturating_add",
    "saturating_sub",
    "saturating_mul",
    "checked_add",
    "checked_sub",
    "checked_mul",
    "checked_div",
    "display",
    "fmt",
    "cmp",
    "partial_cmp",
    "eq",
    "ne",
    "is_some",
    "is_none",
    "is_ok",
    "is_err",
    "unwrap",
    "unwrap_or",
    "unwrap_or_else",
    "unwrap_or_default",
    "expect",
    "map",
    "and_then",
    "or_else",
    "ok_or",
    "ok_or_else",
    "filter",
    "any",
    "all",
    "find",
    "find_map",
    "position",
    "take_while",
    "skip_while",
    "map_while",
    "partition",
    "rposition",
];

const MAP_KEY: &[&str] = &["remove", "contains_key", "get", "get_mut", "get_key_value"];

const STORAGE: &[&str] = &[
    "push",
    "insert",
    "extend",
    "append",
    "add",
    "push_front",
    "push_back",
];

const SLICE_SEARCH: &[&str] = &["contains", "binary_search"];

const TYPE_PRESERVING: &[&str] = &["clone", "to_owned", "to_vec", "into_iter"];

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

pub fn method_mutates_receiver(method: &str) -> bool {
    MUTATES_RECEIVER.contains(&method)
}

pub fn is_known_readonly(method: &str) -> bool {
    KNOWN_READONLY.contains(&method)
}

pub fn is_type_preserving(method: &str) -> bool {
    TYPE_PRESERVING.contains(&method)
}

pub fn is_map_key_method(method: &str) -> bool {
    MAP_KEY.contains(&method)
}

pub fn is_set_lookup_method(method: &str) -> bool {
    matches!(method, "contains" | "remove")
}

pub fn is_collection_key_method(method: &str) -> bool {
    is_map_key_method(method) || is_set_lookup_method(method)
}

pub fn is_storage_method(method: &str) -> bool {
    STORAGE.contains(&method)
}

pub fn is_slice_search_method(method: &str) -> bool {
    SLICE_SEARCH.contains(&method)
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

fn is_map_receiver(receiver_type: Option<&str>) -> bool {
    receiver_type.is_some_and(|ty| {
        let base = ty.split('<').next().unwrap_or(ty);
        MAP_TYPES.contains(&base)
    })
}

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
    if let Some(sig) = lookup_unqualified(method, registry) {
        if sig.has_self_receiver && !sig.param_ownership.is_empty() {
            return sig.param_ownership[0] == OwnershipMode::MutBorrowed;
        }
    }
    false
}

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
    if let Some(sig) = lookup_unqualified(method, registry) {
        if sig.has_self_receiver && !sig.param_ownership.is_empty() {
            return sig.param_ownership[0] != OwnershipMode::MutBorrowed;
        }
    }
    false
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
