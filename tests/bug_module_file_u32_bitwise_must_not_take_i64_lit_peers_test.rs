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

//! Regression: module file u32 bitwise must not take i64 lit peers.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

// Shape: wrapping_* intermediate + annotated
// `let bits: u32 = (x >> 16) & 0x7FFF` inside a non-u32-returning fn. The assign
// slot must keep `_u32` peers — not be overwritten by a default `_i64` peer of `x`.
const SRC: &str = r#"
fn pseudo_random(seed: u32) -> f32 {
    let x = seed.wrapping_mul(1103515245).wrapping_add(12345)
    let bits: u32 = (x >> 16) & 0x7FFF
    bits as f32 / 16383.5 - 1.0
}

pub fn hash_bits(x: u32) -> u32 {
    (x >> 16) & 32767
}
"#;

#[test]
fn module_file_u32_bitwise_must_not_take_i64_lit_peers() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("_i64") || rs.contains("16_i32") || rs.contains("32767_i32");
    assert!(
        !bad,
        "RED: u32 bitwise emitted non-u32 lit peers:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
