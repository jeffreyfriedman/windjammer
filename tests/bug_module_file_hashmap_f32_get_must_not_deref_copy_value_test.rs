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

//! Regression: module file hashmap f32 get must not deref copy value.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
use std::collections::HashMap

pub fn lookup(scores: HashMap<(i32, i32), f32>, x: i32, y: i32) -> f32 {
    let current_g = match scores.get((x, y)) {
        Some(v) => v,
        None => 999999.0,
    }
    current_g
}
"#;

#[test]
fn module_file_hashmap_f32_get_must_not_deref_copy_value() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("Some(v) => *v") || rs.contains("Some(v)=> *v");
    assert!(
        !bad,
        "RED: Copy f32 HashMap get emitted *v deref:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
