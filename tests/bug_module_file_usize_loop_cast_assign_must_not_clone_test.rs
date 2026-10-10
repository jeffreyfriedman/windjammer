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

//! Regression: module file usize loop cast assign must not clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn pick_before(times: Vec<f32>, time: f32) -> i32 {
    let mut before_idx: i32 = 0
    let mut i: usize = 0
    while i < times.len() {
        if times[i] <= time {
            before_idx = i
            break
        }
        i = i + 1
    }
    before_idx
}
"#;

#[test]
fn module_file_usize_loop_cast_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("i.clone()") || rs.contains(".clone() as i32");
    assert!(
        !bad,
        "RED: usize loop counter cloned on cast-assign:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
