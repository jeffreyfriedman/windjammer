#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
    feature = "codegen_tests",
))]

//!
//!   `return Some(String::from("bt: …").to_string())` → redundant coerce.
//! Prefer `Some("bt: …".to_string())` or `Some(String::from("bt: …"))`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn missing_root() -> Option<string> {
    Some("bt: root node record missing")
}

pub fn unknown_child() -> Option<string> {
    Some("bt: child references unknown id")
}
"#;

#[test]
fn module_file_option_string_must_not_string_from_then_to_string() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let chained = rs.contains(").to_string()");
    assert!(
        !(rs.contains("String::from(") && chained),
        "RED: Option<String> emitted String::from(...).to_string():\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

