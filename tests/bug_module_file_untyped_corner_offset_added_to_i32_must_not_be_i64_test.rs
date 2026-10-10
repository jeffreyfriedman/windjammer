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

//! become `i64`.
//!
//!   `let mut i1 = 0_i64` then `ii as i64 + i1` where `ii` is `i32`.
//! WJ is `let mut i1 = 0` and `ii + i1`, and `i1` is also cast with `as f32`.
//! Distinct from (`0` plus `Vec::len()`) and from the neighbor-offset
//! tuple literal that mixes `0_i64` inside `(i32, i32)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn corner(ii: i32, x0: f32, y0: f32) -> f32 {
    let mut i1 = 0
    let mut j1 = 0
    if x0 >= y0 {
        i1 = 1
        j1 = 0
    } else {
        i1 = 0
        j1 = 1
    }
    let shifted = x0 - i1 as f32
    let idx = ii + i1 + j1
    shifted + idx as f32
}
"#;

#[test]
fn module_file_untyped_corner_offset_added_to_i32_must_not_be_i64() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("0_i64") || rs.contains("as i64");
    assert!(
        !bad,
        "RED: untyped corner offset widened to i64:\n{rs}"
    );
    let _ = test.cargo_check();
}

