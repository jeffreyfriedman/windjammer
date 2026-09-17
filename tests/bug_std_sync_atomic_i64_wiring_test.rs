#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! FAILING REPRO — `std::sync::atomic` (AtomicI64) for hot Counter graduation.
//! Ecosystem `wj-sync` Counter is Mutex-backed until runtime re-exports atomics.

#[path = "common/test_utils.rs"]
mod test_utils;

const ATOMIC: &str = r#"
use std::sync::atomic::{AtomicI64, Ordering}

pub fn bump() -> int {
    let n = AtomicI64::new(0)
    n.fetch_add(1, Ordering::Relaxed)
    n.load(Ordering::Relaxed)
}
"#;

#[test]
fn std_sync_atomic_i64_must_wire() {
    let generated = test_utils::compile_single(ATOMIC);
    assert!(
        !generated.contains("compile_error!")
            && !generated.contains("unresolved import")
            && (generated.contains("AtomicI64") || generated.contains("atomic")),
        "std::sync::atomic AtomicI64 must wire for wj-sync Counter hot path:\n{generated}"
    );
    // Runtime re-export must exist (cargo-check style needle).
    assert!(
        generated.contains("std::sync::atomic")
            || generated.contains("windjammer_runtime::sync::atomic"),
        "atomic module path must resolve:\n{generated}"
    );
}
