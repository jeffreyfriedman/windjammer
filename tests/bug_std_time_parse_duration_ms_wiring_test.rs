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

//! Human duration parse (`1h30m`) belongs in `std::time` for `wj-duration` graduation.

#[path = "common/test_utils.rs"]
mod test_utils;

const HUMAN: &str = r#"
use std::time

pub fn parse_human_ms(text: string) -> Result<int, string> {
    time.parse_duration_ms(text)
}

pub fn format_human_ms(ms: int) -> string {
    time.format_duration_ms(ms)
}
"#;

#[test]
fn std_time_parse_duration_ms_must_wire() {
    let generated = test_utils::assert_stdlib_runtime_links(
        HUMAN,
        &[
            "windjammer_runtime::time",
            "parse_duration_ms",
            "format_duration_ms",
        ],
    );
    assert!(
        generated.contains("parse_duration_ms") && generated.contains("format_duration_ms"),
        "std::time duration helpers must reach runtime:\n{generated}"
    );
}
