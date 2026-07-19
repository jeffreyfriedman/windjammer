#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

// WDB-005: if/else u8 branches inside vec! must not emit .clone() on Copy literals.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn if_else_u8_in_vec_literal_no_clone() {
    let source = r#"
const TAG_BOOL: u8 = 1u8

pub fn encode_bool(b: bool) -> Vec<u8> {
    vec![TAG_BOOL, if b { 1u8 } else { 0u8 }]
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        !generated.contains(".clone()"),
        "Copy u8 if/else branches must not use .clone()\nGenerated:\n{}",
        generated
    );
    test_utils::verify_rust_compiles(&generated).expect("generated Rust should compile");
}
