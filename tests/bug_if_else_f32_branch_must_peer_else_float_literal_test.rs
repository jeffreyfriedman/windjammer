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
))]

//! P3.370: if-else assigned to f32-ish slot — else `0.0` must not emit `_f64` when then is f32.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn normalize_mean(count: u32, mut mean_a: f32) -> f32 {
    mean_a = if count > 0 {
        mean_a / count as f32
    } else {
        0.0
    }
    mean_a
}
"#;

#[test]
fn if_else_f32_branch_must_peer_else_float_literal() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.370 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    assert!(
        !rs.contains("0.0_f64"),
        "P3.370: else float must not be f64 when then branch is f32:\n{rs}"
    );
    assert!(
        rs.contains("0.0_f32") || rs.contains("0.0"),
        "P3.370: else float should be f32-compatible:\n{rs}"
    );
    test.cargo_check().expect("P3.370 cargo-check");
}
