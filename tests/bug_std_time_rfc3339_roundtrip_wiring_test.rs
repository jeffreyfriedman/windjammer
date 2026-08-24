//! `std::time.parse_rfc3339` / `to_rfc3339` must wire to runtime.

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
fn std_time_parse_rfc3339_codegen_resolves() {
    let source = r#"
use std::time

pub fn parse_ts(text: string) -> Result<string, string> {
    match time.parse_rfc3339(text) {
        Ok(dt) => Ok(dt.to_rfc3339()),
        Err(e) => Err(e),
    }
}
"#;
    let generated =
        test_utils::assert_stdlib_runtime_links(source, &["parse_rfc3339", "to_rfc3339"]);
    assert!(
        generated.contains("parse_rfc3339") && generated.contains("to_rfc3339"),
        "RFC3339 parse/format must reach runtime time, got:\n{generated}"
    );
}
