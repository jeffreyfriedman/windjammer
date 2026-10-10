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

//! Regression: module file indexed copy u8 vec push must not clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn serialize_f32_bytes(x: f32) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::new()
    let x_bits = x.to_le_bytes()
    bytes.push(x_bits[0])
    bytes.push(x_bits[1])
    bytes.push(x_bits[2])
    bytes.push(x_bits[3])
    bytes
}
"#;

#[test]
fn module_file_indexed_copy_u8_vec_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("x_bits[0].clone()")
        || rs.contains("x_bits[1].clone()")
        || rs.contains("x_bits[2].clone()")
        || rs.contains("x_bits[3].clone()");
    assert!(
        !bad,
        "RED: indexed Copy u8 cloned into Vec::push:\n{rs}"
    );
    let _ = test.cargo_check();
}
