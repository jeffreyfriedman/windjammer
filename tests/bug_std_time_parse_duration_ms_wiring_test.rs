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

//! FAILING REPRO — human duration parse (`1h30m`) belongs in `std::time`.
//!
//! Ecosystem `wj-duration` implements `parse_ms` / `format_ms` in pure WJ.
//! Graduation target: `std::time` Duration parse (or dedicated helper).

#[path = "common/test_utils.rs"]
mod test_utils;

const HUMAN: &str = r#"
use std::time

pub fn parse_human_ms(text: string) -> Result<int, string> {
    time.parse_duration_ms(text)
}
"#;

#[test]
fn std_time_parse_duration_ms_must_wire() {
    let generated = test_utils::compile_single(HUMAN);
    assert!(
        !generated.contains("compile_error!")
            && !generated.contains("unresolved")
            && (generated.contains("parse_duration_ms") || generated.contains("duration")),
        "std::time.parse_duration_ms must be wired for human durations:\n{generated}"
    );
}
