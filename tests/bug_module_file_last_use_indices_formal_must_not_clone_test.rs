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

//! last-use of an owned `Vec` formal into a callee must move, not
//! `indices.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn finish(n: i32, indices: Vec<i32>) -> i32 {
    n + indices.len() as i32
}

pub fn unwrap(positions: Vec<i32>, indices: Vec<i32>) -> i32 {
    let mut i = 0
    while i < indices.len() {
        let _v = indices[i]
        i = i + 1
    }
    finish(positions.len() as i32, indices)
}
"#;

#[test]
fn module_file_last_use_indices_formal_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("indices.clone()");
    assert!(
        !cloned,
        "RED last-use indices formal cloned into callee:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
