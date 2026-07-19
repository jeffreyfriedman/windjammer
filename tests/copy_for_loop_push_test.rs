#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

// Copy element types in `for item in vec` must iterate by value so Vec::push(item) type-checks.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn for_loop_u8_vec_push_by_value() {
    let source = r#"
pub fn append_bytes(encoded: Vec<u8>, mut out: Vec<u8>) -> Vec<u8> {
    for byte in encoded {
        out.push(byte)
    }
    out
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        !generated.contains("push(*byte)") && !generated.contains("push(&byte)"),
        "u8 loop variable must push by value, not deref/borrow.\nGenerated:\n{generated}"
    );
    assert!(
        generated.contains("push(byte)"),
        "expected push(byte).\nGenerated:\n{generated}"
    );
    test_utils::verify_rust_compiles(&generated).expect("generated Rust should compile");
}
