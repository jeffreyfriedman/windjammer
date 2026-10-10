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

//! Regression: module file copy usize cast must not emit clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn collect_slots(keys: Vec<i32>) -> Vec<usize> {
    let mut slots: Vec<usize> = Vec::new()
    for key in keys {
        slots.push(key as usize)
    }
    slots
}
"#;

#[test]
fn module_file_copy_usize_cast_must_not_emit_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()");
    assert!(
        !bad,
        "RED: Copy usize cast emitted .clone():\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
