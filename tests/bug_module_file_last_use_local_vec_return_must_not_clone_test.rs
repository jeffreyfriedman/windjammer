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

//! Regression: module file last use local vec return must not clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn leaf(node: u32) -> Vec<u32> {
    let mut result = Vec::new()
    result.push(node)
    return result
}

pub fn empty_path() -> Vec<i32> {
    let mut result = Vec::new()
    if true {
        return result
    }
    result.push(1)
    result
}
"#;

#[test]
fn module_file_last_use_local_vec_return_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("result.clone()");
    assert!(
        !cloned,
        "RED: last-use local Vec return cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
