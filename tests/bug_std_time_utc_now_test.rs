//! `std::time.utc_now()` must codegen to runtime clock and `cargo check`.

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
    test_utils::assert_stdlib_runtime_links_any(source, &["time::utc_now", "time::now"]);
}
