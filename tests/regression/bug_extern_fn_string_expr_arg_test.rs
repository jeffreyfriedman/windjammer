#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

#[path = "../common/test_utils.rs"]
mod test_utils;

/// TDD: game code must not write `unsafe` for extern fn calls — compiler auto-wraps FFI.
/// Also verifies string expression args compile through string_to_ffi.
#[test]
fn test_extern_fn_accepts_string_expression_without_unsafe_in_source() {
    let source = r#"
extern fn emit_line(text: string)

fn build_message() -> string {
    let mut s = "frame=1".to_string()
    s = format!("{};hp=100", s)
    s
}

pub fn emit_observation() {
    emit_line(build_message())
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("string_to_ffi"),
        "extern fn string arg should use string_to_ffi wrapper. Got:\n{}",
        generated
    );
    assert!(
        generated.contains("emit_line("),
        "should call emit_line. Got:\n{}",
        generated
    );
    assert!(
        !generated.contains("build_message().to_string().to_string()"),
        "should not double-convert string expression. Got:\n{}",
        generated
    );
}
