//! The memoized stdlib tables (taint sources, taint sinks, effect declarations) are exactly the
//! unmemoized computation: same constraints, same order, on every call.
//! Run: cargo test --release --test stdlib_tables_memo

use windjammer::ir::effects::{compute_stdlib_effect_declarations, stdlib_effect_declarations};
use windjammer::ir::taint::{compute_stdlib_sinks, compute_stdlib_taint_sources, stdlib_sinks, stdlib_taint_sources};

fn dbg<T: std::fmt::Debug>(v: &[T]) -> Vec<String> {
    v.iter().map(|c| format!("{c:?}")).collect()
}

#[test]
fn taint_sources_memo_equals_computation() {
    let fresh = dbg(&compute_stdlib_taint_sources());
    assert!(!fresh.is_empty(), "the stdlib declares taint sources (env, fs, io)");
    assert_eq!(dbg(&stdlib_taint_sources()), fresh);
    assert_eq!(dbg(&stdlib_taint_sources()), fresh, "second (memoized) call");
}

#[test]
fn taint_sinks_memo_equals_computation() {
    let fresh = dbg(&compute_stdlib_sinks());
    assert!(!fresh.is_empty(), "the stdlib declares sinks (process spawn, fs write)");
    assert_eq!(dbg(&stdlib_sinks()), fresh);
    assert_eq!(dbg(&stdlib_sinks()), fresh, "second (memoized) call");
}

#[test]
fn effect_declarations_memo_equals_computation() {
    let fresh = dbg(&compute_stdlib_effect_declarations());
    assert!(!fresh.is_empty());
    assert_eq!(dbg(&stdlib_effect_declarations()), fresh);
    assert_eq!(dbg(&stdlib_effect_declarations()), fresh, "second (memoized) call");
}
