#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "parser_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

/// Large unsigned hex constants (e.g. sign-bit set) must lex and compile for u64 targets.
#[test]
fn test_hex_literal_u64_sign_bit_compiles() {
    let source = r#"
pub const XOR_MASK: u64 = 0x8000000000000000u64

pub fn mask_value() -> u64 {
    XOR_MASK
}
"#;

    let (generated, compiles) = test_utils::compile_single_check(source);
    assert!(
        compiles,
        "u64 hex literal with sign bit set should compile.\nGenerated:\n{generated}"
    );
    assert!(
        generated.contains("0x8000000000000000u64")
            || generated.contains("9223372036854775808u64")
            || generated.contains("-9223372036854775808_u64")
            || generated.contains("i64::MIN as u64"),
        "Expected large u64 hex constant in output.\nGenerated:\n{generated}"
    );
}

#[test]
fn test_hex_literal_i64_min_bitpattern() {
    let source = r#"
pub fn min_i64_bits() -> i64 {
    0x8000000000000000
}
"#;

    let (generated, compiles) = test_utils::compile_single_check(source);
    assert!(
        compiles,
        "0x8000000000000000 should map to i64::MIN bit pattern.\nGenerated:\n{generated}"
    );
}
