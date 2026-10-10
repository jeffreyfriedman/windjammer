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

//! Regression: module file copy local in println must not clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn log_size(w: i32, h: i32, d: i32) {
    let max_size = w
    println("grid {}x{}x{} size={}", w, h, d, max_size)
}
"#;

#[test]
fn module_file_copy_local_in_println_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains(".clone()");
    assert!(
        !cloned,
        "RED: Copy local cloned in println/format:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
