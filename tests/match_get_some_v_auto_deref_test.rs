#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! TDD: `match map.get(k) { Some(v) => v, None => lit }` must deref `v` when get returns Option<&V>.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_match_get_some_v_without_explicit_deref() {
    let source = r#"use std::collections::HashMap

pub fn score(g_score: HashMap<u32, f32>, key: u32) -> f32 {
    match g_score.get(key) {
        Some(v) => v,
        None => 999999.0,
    }
}
"#;

    let rust_code = test_utils::compile_single(source);
    assert!(
        rust_code.contains("Some(v) => *v") || rust_code.contains("Some(v) => * v"),
        "Some(v) arm should deref borrowed get() value, got:\n{rust_code}"
    );
    test_utils::verify_rust_compiles(&rust_code).expect("generated match should compile");
}
