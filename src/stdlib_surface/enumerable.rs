//! The enumerable collection surface (ADR-0018 feature 1, spec area COLL), declared once.
//!
//! Every backend asks this table instead of matching method names itself: the Rust backend for the runtime
//! name, the portable typer (Go/JS/TS) for closure parameter and result types, the interpreter for the
//! method list it must implement (checked by a test). Signatures are also published for humans and agents in
//! `std/enumerable.wj`; `tests/enumerable_surface_test.rs` keeps the two in sync.

/// Shape of a method's result, relative to the element type `T` of the receiver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ret {
    /// `Map<K, List<T>>`, `K` = the key closure's result.
    GroupMap,
    /// `Map<K, int>`.
    CountMap,
    /// `Map<T, int>`.
    TallyMap,
    /// The closure's result type (`sum_by`).
    ClosureResult,
    /// `List<T>`.
    List,
    /// `List<List<T>>`.
    ListOfLists,
    /// `(List<T>, List<T>)`.
    ListPair,
    /// `List<R>`, `R` = the closure's result (`zip_with`).
    ClosureList,
    /// `int?` (an index).
    IndexOpt,
    /// `()` (mutates the receiver).
    Unit,
}

/// What argument `i` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arg {
    /// A closure `(T) -> K` (key / projection / predicate; receives the element by reference).
    KeyFn,
    /// A closure `(T, U) -> R` whose second parameter is the other sequence's element (`zip_with`).
    Zip2Fn,
    /// A value of the element type `T`.
    Elem,
    /// A count (`int`, converted to the backend's size type).
    Count,
    /// Another sequence.
    Seq,
}

#[derive(Debug, Clone, Copy)]
pub struct Method {
    /// Windjammer name.
    pub name: &'static str,
    /// Arguments, in order.
    pub args: &'static [Arg],
    pub ret: Ret,
    /// Name in the Rust runtime (`windjammer_runtime::enumerable`, inlined as `wj_enumerable`).
    pub rust: &'static str,
    /// Mutates the receiver in place (needs a mutable receiver).
    pub mutating: bool,
}

const fn m(name: &'static str, args: &'static [Arg], ret: Ret, rust: &'static str) -> Method {
    Method {
        name,
        args,
        ret,
        rust,
        mutating: false,
    }
}

use Arg::*;

/// The surface. Methods that Rust's std already provides with the same meaning on every backend
/// (`flat_map`, `scan`, `take_while`, `skip_while`, `windows`, `chunks`, `sort_by_key`, `first`, `last`, `nth`,
/// `min_by_key`, `max_by_key`, `partition`) are not repeated here; see `docs/spec/collections.md`.
pub const METHODS: &[Method] = &[
    m("group_by", &[KeyFn], Ret::GroupMap, "wj_group_by"),
    m("count_by", &[KeyFn], Ret::CountMap, "wj_count_by"),
    m("tally", &[], Ret::TallyMap, "wj_tally"),
    m("frequencies", &[], Ret::TallyMap, "wj_tally"),
    m("sum_by", &[KeyFn], Ret::ClosureResult, "wj_sum_by"),
    m("unique", &[], Ret::List, "wj_unique"),
    m("unique_by", &[KeyFn], Ret::List, "wj_unique_by"),
    m("chunk_by", &[KeyFn], Ret::ListOfLists, "wj_chunk_by"),
    m("each_slice", &[Count], Ret::ListOfLists, "wj_each_slice"),
    m("partition_by", &[KeyFn], Ret::ListPair, "wj_partition_by"),
    m("intersperse", &[Elem], Ret::List, "wj_intersperse"),
    m("zip_with", &[Seq, Zip2Fn], Ret::ClosureList, "wj_zip_with"),
    m("index_of", &[Elem], Ret::IndexOpt, "wj_index_of"),
    m("sorted", &[], Ret::List, "wj_sorted"),
    m("sorted_by_key", &[KeyFn], Ret::List, "wj_sorted_by_key"),
    m(
        "sorted_by_key_desc",
        &[KeyFn],
        Ret::List,
        "wj_sorted_by_key_desc",
    ),
    m("to_list", &[], Ret::List, "wj_to_list"),
    Method {
        name: "sort_by_key_desc",
        args: &[KeyFn],
        ret: Ret::Unit,
        rust: "wj_sort_by_key_desc",
        mutating: true,
    },
];

/// The surface entry for `name` called with `argc` arguments, if any.
pub fn lookup(name: &str, argc: usize) -> Option<&'static Method> {
    METHODS
        .iter()
        .find(|m| m.name == name && m.args.len() == argc)
}

/// Is `name` a surface method (any arity)? Used for arity diagnostics.
pub fn is_surface_name(name: &str) -> bool {
    METHODS.iter().any(|m| m.name == name)
}

/// Name of the inlined Rust module holding the implementations.
pub const RUST_MODULE: &str = "wj_enumerable";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_per_arity() {
        for (i, a) in METHODS.iter().enumerate() {
            for b in &METHODS[i + 1..] {
                assert!(
                    !(a.name == b.name && a.args.len() == b.args.len()),
                    "duplicate {}",
                    a.name
                );
            }
        }
    }

    #[test]
    fn rust_names_are_prefixed() {
        assert!(METHODS.iter().all(|m| m.rust.starts_with("wj_")));
        assert_eq!(lookup("group_by", 1).map(|m| m.rust), Some("wj_group_by"));
        assert!(lookup("group_by", 2).is_none());
    }
}
