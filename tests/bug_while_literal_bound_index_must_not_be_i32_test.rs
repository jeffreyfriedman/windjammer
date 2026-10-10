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

//! `while i < 6` used as an array/vec index must not be `i32`.
//!
//! `fps_camera.wj` writes `let mut i = 0` then `while i < 6` and `offsets[i]`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_i32_index(body: &str) -> bool {
    body.contains("i = 0_i32") || body.contains("6_i32") || body.contains("i as usize")
}

#[test]
fn while_literal_bound_index_must_not_be_i32() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub fn pick(offsets: Vec<f32>) -> f32 {
    let mut i = 0
    let mut best = 0.0
    while i < 6 {
        best = offsets[i]
        i = i + 1
    }
    best
}
"#,
    );
    let map = test
        .compile()
        .expect("while literal-bound index must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_i32_index(body),
        "while i < 6 used as an index must not be i32; got:\n{body}"
    );
}
