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

//!
//!   `from_le_bytes(zero_bits.clone())` then `from_le_bytes(zero_bits)`
//! WJ uses bare `zero_bits` both times.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn pack_zeros() -> Vec<f32> {
    let mut data: Vec<f32> = Vec::new()
    let zero_bits = 0u32.to_le_bytes()
    data.push(f32::from_le_bytes(zero_bits))
    data.push(f32::from_le_bytes(zero_bits))
    data
}
"#;

#[test]
fn module_file_copy_byte_array_reuse_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("zero_bits.clone()");
    assert!(
        !bad,
        "RED: Copy byte array cloned into from_le_bytes:\n{rs}"
    );
    let _ = test.cargo_check();
}

