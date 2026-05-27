//! Centralized, data-driven registry of known standard library method behaviors.
//!
//! This module is the **single source of truth** for stdlib method properties:
//! mutability, iterator returns, map-key semantics, closure arguments, etc.
//!
//! ALL code that needs to know about method behavior MUST query this module
//! instead of scattering hardcoded string-matching heuristics. This eliminates:
//! - Duplicate, divergent method lists across 6+ files
//! - Fragile prefix/suffix heuristics (`starts_with("set")`, `ends_with("_mut")`)
//! - Game-specific hardcoded names ("damage", "smooth_follow", "look_at")
//!
//! For user-defined methods, behavior comes from analyzing their actual source
//! code via the SignatureRegistry — not from guessing based on their name.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

const MUTATES_RECEIVER: u16 = 0b0000_0001;
const RETURNS_ITERATOR: u16 = 0b0000_0010;
const MAP_KEY_METHOD: u16 = 0b0000_0100;
const INDEX_TAKING: u16 = 0b0000_1000;
const CLOSURE_TAKING: u16 = 0b0001_0000;
const COMMON_STDLIB_NAME: u16 = 0b0010_0000;
const STORES_PARAMETER: u16 = 0b0100_0000;

struct MethodEntry {
    name: &'static str,
    traits: u16,
}

const fn m(name: &'static str, traits: u16) -> MethodEntry {
    MethodEntry { name, traits }
}

const MUT: u16 = MUTATES_RECEIVER;
const ITER: u16 = RETURNS_ITERATOR;
const MAP_KEY: u16 = MAP_KEY_METHOD;
const IDX: u16 = INDEX_TAKING;
const CLOSURE: u16 = CLOSURE_TAKING;
const COMMON: u16 = COMMON_STDLIB_NAME;
const STORE: u16 = STORES_PARAMETER;

/// The canonical table of known stdlib method behaviors.
///
/// To add a new stdlib method, add ONE entry here. That's it.
/// All downstream code automatically picks it up.
static KNOWN_METHODS: &[MethodEntry] = &[
    // ── Vec / VecDeque / collections: mutating ──
    m("push", MUT | COMMON | STORE),
    m("pop", MUT | COMMON),
    m("insert", MUT | COMMON | IDX | STORE),
    m("remove", MUT | COMMON | MAP_KEY | IDX),
    m("clear", MUT | COMMON),
    m("append", MUT | STORE),
    m("extend", MUT | STORE),
    m("drain", MUT | ITER | IDX),
    m("truncate", MUT),
    m("resize", MUT),
    m("retain", MUT),
    m("sort", MUT),
    m("sort_by", MUT),
    m("sort_by_key", MUT),
    m("sort_unstable", MUT),
    m("sort_unstable_by", MUT),
    m("dedup", MUT),
    m("reverse", MUT),
    m("swap", MUT | IDX),
    m("swap_remove", MUT | IDX),
    m("reserve", MUT),
    m("shrink_to_fit", MUT),
    m("split_off", MUT | IDX),
    m("fill", MUT),
    m("set", MUT | COMMON),
    m("rotate_left", MUT),
    m("rotate_right", MUT),
    m("set_len", MUT),
    m("push_str", MUT | STORE),
    // ── Game engine render paths (dogfooding) ──
    m("render_frame", MUT),
    m("render_frame_with_dt", MUT),
    m("update_camera", MUT),
    m("rebuild_shader_graph", MUT),
    m("push_front", MUT | STORE),
    m("push_back", MUT | STORE),
    m("pop_front", MUT),
    m("pop_back", MUT),
    m("make_ascii_lowercase", MUT),
    m("make_ascii_uppercase", MUT),
    m("add", MUT | STORE),
    // ── Option / Result: mutating ──
    m("take", MUT),
    m("replace", MUT),
    m("get_or_insert", MUT),
    m("get_or_insert_with", MUT),
    // ── HashMap / BTreeMap: mutating + key methods ──
    m("entry", MUT),
    // ── HashMap / BTreeMap: read-only key methods ──
    m("contains_key", COMMON | MAP_KEY),
    m("get", COMMON | MAP_KEY),
    m("get_mut", MUT | MAP_KEY | COMMON),
    m("get_key_value", MAP_KEY),
    // ── Mutating iterator variants ──
    m("iter_mut", MUT | ITER),
    m("values_mut", MUT),
    // ── Collection inspection: read-only ──
    m("len", COMMON),
    m("is_empty", COMMON),
    m("contains", COMMON),
    m("first", COMMON),
    m("last", COMMON),
    m("capacity", 0),
    m("keys", ITER | COMMON),
    m("values", ITER | COMMON),
    m("binary_search", COMMON),
    m("to_le_bytes", COMMON),
    m("to_be_bytes", COMMON),
    m("from_le_bytes", COMMON),
    m("from_be_bytes", COMMON),
    // ── Iterators: read-only ──
    m("iter", ITER | COMMON),
    m("into_iter", ITER),
    m("windows", 0),
    m("chunks", 0),
    m("enumerate", 0),
    m("lines", ITER),
    m("chars", ITER),
    m("bytes", ITER),
    m("split", ITER),
    m("split_whitespace", ITER),
    // ── Cloning / conversion: read-only ──
    m("clone", 0),
    m("to_string", 0),
    m("to_owned", 0),
    m("to_vec", 0),
    m("as_str", 0),
    m("as_ref", 0),
    m("as_slice", 0),
    m("as_bytes", 0),
    m("as_deref", 0),
    m("as_ptr", 0),
    m("as_mut_ptr", 0),
    // ── String inspection: read-only ──
    m("trim", 0),
    m("starts_with", COMMON),
    m("ends_with", COMMON),
    m("to_lowercase", 0),
    m("to_uppercase", 0),
    m("is_ascii", 0),
    m("substring", 0),
    m("splitn", 0),
    m("rsplit", 0),
    m("repeat", 0),
    m("replacen", 0),
    m("rfind", 0),
    m("match_indices", 0),
    m("trim_start", 0),
    m("trim_end", 0),
    m("strip_prefix", 0),
    m("strip_suffix", 0),
    m("to_ascii_lowercase", 0),
    m("to_ascii_uppercase", 0),
    // ── Numeric: read-only (Copy types) ──
    m("abs", 0),
    m("ceil", 0),
    m("floor", 0),
    m("round", 0),
    m("sqrt", 0),
    m("cbrt", 0),
    m("powi", 0),
    m("powf", 0),
    m("sin", 0),
    m("cos", 0),
    m("tan", 0),
    m("asin", 0),
    m("acos", 0),
    m("atan", 0),
    m("atan2", 0),
    m("sinh", 0),
    m("cosh", 0),
    m("tanh", 0),
    m("asinh", 0),
    m("acosh", 0),
    m("atanh", 0),
    m("log", 0),
    m("log2", 0),
    m("log10", 0),
    m("ln", 0),
    m("exp", 0),
    m("exp2", 0),
    m("min", 0),
    m("max", 0),
    m("clamp", 0),
    m("signum", 0),
    m("copysign", 0),
    m("fract", 0),
    m("recip", 0),
    m("to_radians", 0),
    m("to_degrees", 0),
    m("is_nan", 0),
    m("is_infinite", 0),
    m("is_finite", 0),
    m("is_normal", 0),
    m("is_sign_positive", 0),
    m("is_sign_negative", 0),
    m("leading_zeros", 0),
    m("trailing_zeros", 0),
    m("count_ones", 0),
    m("count_zeros", 0),
    m("wrapping_add", 0),
    m("wrapping_sub", 0),
    m("wrapping_mul", 0),
    m("saturating_add", 0),
    m("saturating_sub", 0),
    m("saturating_mul", 0),
    m("checked_add", 0),
    m("checked_sub", 0),
    m("checked_mul", 0),
    m("checked_div", 0),
    // ── Vector / geometry / bounds: read-only ──
    m("normalize", 0),
    m("length", 0),
    m("length_squared", 0),
    m("dot", 0),
    m("cross", 0),
    m("distance", 0),
    m("lerp", 0),
    m("midpoint", 0),
    m("magnitude", 0),
    m("center", 0),
    m("size", 0),
    m("extents", 0),
    m("min_point", 0),
    m("max_point", 0),
    m("width", 0),
    m("height", 0),
    m("depth", 0),
    m("area", 0),
    m("volume", 0),
    m("intersects", 0),
    m("overlaps", 0),
    m("to_column_major_array", 0),
    m("to_row_major_array", 0),
    m("transpose", 0),
    m("inverse", 0),
    m("determinant", 0),
    m("identity", 0),
    m("multiply", 0),
    m("scale", 0),
    m("rotate", 0),
    m("translate", 0),
    m("transform", 0),
    m("apply", 0),
    m("view_projection_matrix", 0),
    m("get_frustum_planes", 0),
    m("pack_camera_uniforms", 0),
    m("perspective", 0),
    m("look_at", 0),
    m("orthographic", 0),
    m("squared", 0),
    m("pow", 0),
    // ── Display / formatting: read-only ──
    m("display", 0),
    m("fmt", 0),
    // ── Comparison: read-only ──
    m("cmp", 0),
    m("partial_cmp", 0),
    m("eq", 0),
    m("ne", 0),
    // ── Option / Result inspection: read-only ──
    m("is_some", 0),
    m("is_none", 0),
    m("is_ok", 0),
    m("is_err", 0),
    m("unwrap", 0),
    m("unwrap_or", 0),
    m("unwrap_or_else", 0),
    m("unwrap_or_default", 0),
    m("expect", 0),
    m("map", 0),
    m("and_then", 0),
    m("or_else", 0),
    m("ok_or", 0),
    m("ok_or_else", 0),
    // ── Closure-taking iterator methods ──
    m("filter", CLOSURE),
    m("any", CLOSURE),
    m("all", CLOSURE),
    m("find", CLOSURE),
    m("find_map", CLOSURE),
    m("position", CLOSURE),
    m("take_while", CLOSURE),
    m("skip_while", CLOSURE),
    m("map_while", CLOSURE),
    m("partition", CLOSURE),
    m("rposition", CLOSURE),
];

static REGISTRY: LazyLock<HashMap<&'static str, u16>> = LazyLock::new(|| {
    let mut map = HashMap::with_capacity(KNOWN_METHODS.len());
    for entry in KNOWN_METHODS {
        map.insert(entry.name, entry.traits);
    }
    map
});

static KNOWN_NAMES: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| KNOWN_METHODS.iter().map(|e| e.name).collect());

fn get_traits(method: &str) -> Option<u16> {
    REGISTRY.get(method).copied()
}

/// Returns `true` if `method` is a known stdlib method that mutates its receiver (`&mut self`).
///
/// Returns `false` for unknown methods — the caller should consult the
/// `SignatureRegistry` for user-defined method signatures instead of guessing.
pub fn mutates_receiver(method: &str) -> bool {
    get_traits(method).is_some_and(|t| t & MUTATES_RECEIVER != 0)
}

/// Returns `true` if `method` is a known stdlib method that returns an iterator.
pub fn returns_iterator(method: &str) -> bool {
    get_traits(method).is_some_and(|t| t & RETURNS_ITERATOR != 0)
}

/// Returns `true` if `method` is a HashMap/BTreeMap key method whose first
/// argument should be treated as the key type (affects auto-borrowing).
pub fn is_map_key_method(method: &str) -> bool {
    get_traits(method).is_some_and(|t| t & MAP_KEY_METHOD != 0)
}

/// Returns `true` if `method` takes an index argument (first arg is usize).
pub fn is_index_taking_method(method: &str) -> bool {
    get_traits(method).is_some_and(|t| t & INDEX_TAKING != 0)
}

/// Returns `true` if `method` takes a closure/predicate as its first argument.
pub fn is_closure_taking_method(method: &str) -> bool {
    get_traits(method).is_some_and(|t| t & CLOSURE_TAKING != 0)
}

/// Returns `true` if `method` is a common stdlib method name that is too
/// ambiguous for unqualified signature registry lookup.
pub fn is_common_stdlib_method(method: &str) -> bool {
    get_traits(method).is_some_and(|t| t & COMMON_STDLIB_NAME != 0)
}

/// Returns `true` if `method` is ANY known stdlib method (mutating or read-only).
pub fn is_known_stdlib_method(method: &str) -> bool {
    KNOWN_NAMES.contains(method)
}

/// Returns `true` if `method` is a storage method that moves a parameter value
/// into a collection (push, insert, extend, append, etc.).
pub fn is_storage_method(method: &str) -> bool {
    get_traits(method).is_some_and(|t| t & STORES_PARAMETER != 0)
}

/// Returns `true` if `method` is a known read-only stdlib method (takes `&self`, not `&mut self`).
pub fn is_known_readonly_method(method: &str) -> bool {
    match get_traits(method) {
        Some(traits) => traits & MUTATES_RECEIVER == 0,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_duplicates_in_known_methods() {
        let mut seen = HashSet::new();
        for entry in KNOWN_METHODS {
            assert!(
                seen.insert(entry.name),
                "Duplicate method '{}' in KNOWN_METHODS table",
                entry.name
            );
        }
    }

    #[test]
    fn test_registry_populated() {
        assert!(REGISTRY.len() > 100, "Registry should have 100+ methods");
    }

    #[test]
    fn test_mutating_and_readonly_are_disjoint() {
        for entry in KNOWN_METHODS {
            if entry.traits & MUTATES_RECEIVER != 0 {
                assert!(
                    !is_known_readonly_method(entry.name),
                    "'{}' cannot be both mutating and read-only",
                    entry.name
                );
            }
        }
    }
}
