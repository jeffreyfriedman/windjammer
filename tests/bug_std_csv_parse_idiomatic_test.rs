//! `std::csv.parse` must expose idiomatic WJ `Result<…, string>` and `cargo check`.

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
    let generated = test_utils::assert_stdlib_runtime_links(source, &["csv::parse"]);
    assert!(
        !generated.contains("csv.Error"),
        "must not leak Rust csv.Error into WJ surface, got:\n{generated}"
    );
}
