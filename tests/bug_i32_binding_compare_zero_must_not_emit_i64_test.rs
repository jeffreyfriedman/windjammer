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

//! P3.750: an `i32` binding compared to untyped `0` must not emit `0_i64`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_zero(body: &str) -> bool {
    body.contains(">= 0_i64") || body.contains(">= 0_i64)")
}

#[test]
fn i32_binding_compare_zero_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub fn grouped() -> bool {
    let mut existing_group: i32 = -1
    existing_group >= 0
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.750: i32 compare-zero fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_zero(body),
        "i32 binding compared to 0 must not use 0_i64; got:\n{body}"
    );
}
