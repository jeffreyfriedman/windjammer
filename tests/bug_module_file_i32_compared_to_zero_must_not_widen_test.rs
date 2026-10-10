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

//! Regression: module file i32 compared to zero must not widen.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn abs_perm(seed: i64, i: i32) -> i32 {
    let val = ((seed * (i as i64 + 1)) % 256) as i32
    if val < 0 { -val } else { val }
}
"#;

#[test]
fn module_file_i32_compared_to_zero_must_not_widen() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("0_i64");
    assert!(
        !bad,
        "RED: i32 compared to a widened zero:\n{rs}"
    );
    let _ = test.cargo_check();
}
