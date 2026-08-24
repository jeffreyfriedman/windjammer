//! FAILING REPRO — `std::csv` must expose idiomatic Windjammer `Result<…, string>`.
//!
//! Current `std/csv.wj` leaks Rust `csv.Error` and non-WJ syntax. Ecosystem needs:
//! `csv.parse(text) -> Result<Vec<Vec<string>>, string>`.

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
fn std_csv_parse_idiomatic_codegen_resolves() {
    let source = r#"
use std::csv

pub fn rows(text: string) -> Result<Vec<Vec<string>>, string> {
    csv.parse(text)
}
"#;
    let (generated, ok) = test_utils::compile_single_check(source);
    assert!(
        ok,
        "std::csv.parse must compile with WJ Result API, got:\n{generated}"
    );
    let wired = generated.contains("csv::parse")
        || generated.contains("csv_mod::parse")
        || generated.contains("windjammer_runtime::csv");
    assert!(
        wired,
        "csv.parse must map to runtime csv, got:\n{generated}"
    );
    assert!(
        !generated.contains("csv.Error"),
        "must not leak Rust csv.Error into WJ surface, got:\n{generated}"
    );
}
