//! The session-shared stdlib tables (`ir::stdlib_tables::StdlibTables`) are exactly what each IR pipeline computed
//! for itself before: same constraints, same order, and the same on every computation.
//! Run: cargo test --release --test stdlib_tables

use windjammer::ir::effects::stdlib_effect_declarations;
use windjammer::ir::stdlib_tables::StdlibTables;
use windjammer::ir::taint::{stdlib_sinks, stdlib_taint_sources};

fn dbg<T: std::fmt::Debug>(v: &[T]) -> Vec<String> {
    v.iter().map(|c| format!("{c:?}")).collect()
}

#[test]
fn shared_tables_equal_the_per_pipeline_computation() {
    let t = StdlibTables::compute();
    assert!(!t.taint_sources.is_empty(), "the stdlib declares taint sources (env, fs, io)");
    assert!(!t.taint_sinks.is_empty(), "the stdlib declares sinks (process spawn, fs write)");
    assert!(!t.effect_declarations.is_empty());
    assert_eq!(dbg(&t.taint_sources), dbg(&stdlib_taint_sources()));
    assert_eq!(dbg(&t.taint_sinks), dbg(&stdlib_sinks()));
    assert_eq!(dbg(&t.effect_declarations), dbg(&stdlib_effect_declarations()));
}

#[test]
fn tables_are_deterministic_across_computations() {
    let (a, b) = (StdlibTables::compute(), StdlibTables::compute());
    assert_eq!(dbg(&a.taint_sources), dbg(&b.taint_sources));
    assert_eq!(dbg(&a.taint_sinks), dbg(&b.taint_sinks));
    assert_eq!(dbg(&a.effect_declarations), dbg(&b.effect_declarations));
}
