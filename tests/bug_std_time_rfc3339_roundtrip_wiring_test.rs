//! FAILING REPRO — `std::time.parse_rfc3339` / `to_rfc3339` must wire to runtime.
//!
//! Ecosystem `wj-timefmt` is a pure-WJ Zulu subset. Std should own RFC3339.

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
    let (generated, ok) = test_utils::compile_single_check(source);
    assert!(
        ok,
        "std::time.parse_rfc3339/to_rfc3339 must compile, got:\n{generated}"
    );
    let parse_ok = generated.contains("parse_rfc3339")
        || generated.contains("time::parse_rfc3339");
    let format_ok = generated.contains("to_rfc3339");
    assert!(
        parse_ok && format_ok,
        "RFC3339 parse/format must reach runtime time, got:\n{generated}"
    );
}
