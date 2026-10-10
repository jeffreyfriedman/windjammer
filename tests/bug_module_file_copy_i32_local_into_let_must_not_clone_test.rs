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

//! Regression: module file copy i32 local into let must not clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn ground_probe(pos_y: f32, size_y: f32) -> i32 {
    let min_y = (pos_y - size_y / 2.0) as i32
    let max_y = (pos_y + size_y / 2.0) as i32
    let feet_y = min_y
    let head_y = max_y
    feet_y + head_y
}
"#;

#[test]
fn module_file_copy_i32_local_into_let_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("min_y.clone()") || rs.contains("max_y.clone()");
    assert!(
        !bad,
        "RED: Copy i32 local cloned into untyped let:\n{rs}"
    );
    let _ = test.cargo_check();
}
