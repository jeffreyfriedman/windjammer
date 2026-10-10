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

//! `parse_flag(line.clone(), …)` at every callsite.
//!
//!   `parse_flag(line.clone(), "forward")` then `parse_flag(line.clone(), "back")` …
//! WJ is `parse_flag(line, "forward")` / `parse_flag(line, "back")`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn parse_flag(line: string, key: string) -> bool {
    if line.len() == 0 {
        false
    } else {
        key.len() > 0
    }
}

pub fn from_line(line: string) -> i32 {
    let f = parse_flag(line, "forward")
    let b = parse_flag(line, "back")
    if f {
        1
    } else if b {
        2
    } else {
        0
    }
}
"#;

#[test]
fn module_file_reused_string_must_not_clone_each_parse_flag() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("line.clone()");
    assert!(
        !cloned,
        "RED: reused string cloned at each parse_flag callsite:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

