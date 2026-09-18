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

//! P3.374: `let x = if c { 1.0 / sx } else { 0.0 }` — else must peer f32 when then is f32
//! (skeleton.wj inverse scale; ~36× E0308 expected f32 found f64 on else `_f64`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn inverse_scale(sx: f32) -> f32 {
    let isx = if sx > 0.0001 {
        1.0 / sx
    } else {
        0.0
    }
    isx
}
"#;

#[test]
fn let_if_else_f32_branch_must_peer_else_float_literal() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.374 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    assert!(
        !rs.contains("0.0_f64"),
        "P3.374: else float must not be f64 when then branch is f32:\n{rs}"
    );
    assert!(
        rs.contains("0.0_f32") || rs.contains("0.0"),
        "P3.374: else float should be f32-compatible:\n{rs}"
    );
    test.cargo_check().expect("P3.374 cargo-check");
}
