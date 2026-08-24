//! `std::time.utc_now().timestamp_millis()` must reach runtime clock.

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
    test_utils::assert_stdlib_runtime_links_any(
        source,
        &["timestamp_millis", "time::utc_now", "time::now"],
    );
}
