//! `std::time.utc_now().timestamp_millis()` must reach a real clock (not stub `0`).
//!
//! Ecosystem `wj-uuid`: `v1()` uses current Unix milliseconds for time-based UUIDs.

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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn std_time_utc_now_timestamp_millis_codegen() {
    let source = r#"
use std::time

pub fn now_millis() -> i64 {
    time.utc_now().timestamp_millis()
}
"#;
    let generated = test_utils::compile_single(source);
    let wired = generated.contains("now_millis()")
        || generated.contains("timestamp_millis()")
        || generated.contains("time::now_millis()");
    assert!(
        wired,
        "utc_now timestamp_millis must codegen to runtime clock, got:\n{generated}"
    );
    assert!(
        !generated.contains("cannot find function `timestamp_millis`"),
        "must not emit missing timestamp_millis on DateTime, got:\n{generated}"
    );
}
