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

//! `std::sync::atomic` AtomicI64 must link for wj-sync Counter hot path.

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
    test_utils::assert_stdlib_runtime_links(
        ATOMIC,
        &["AtomicI64", "fetch_add"],
    );
}
