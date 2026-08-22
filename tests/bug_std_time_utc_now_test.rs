//! `std::time.utc_now()` must codegen to a real runtime clock function.
//!
//! Ecosystem `wj-uuid`: `v1()` calls `time.utc_now().timestamp_millis()`.

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
fn std_time_utc_now_codegen() {
    let source = r#"
use std::time

pub fn now() -> time.DateTime {
    time.utc_now()
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("time::utc_now")
            || generated.contains("time::now")
            || generated.contains("Utc::now"),
        "std::time.utc_now must codegen to runtime clock, got:\n{generated}"
    );
    assert!(
        !generated.contains("cannot find function `utc_now`"),
        "must not emit missing utc_now, got:\n{generated}"
    );
}
