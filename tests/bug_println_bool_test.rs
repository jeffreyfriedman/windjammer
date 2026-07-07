#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

/// TDD: println with non-string arguments (standalone build uses Rust macro).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_println_bool() {
    let source = r#"
fn main() {
    let value = true
    println(value)
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("println!(\"{}\", value)"),
        "standalone println(bool) should use format macro. Generated:\n{generated}"
    );
}
