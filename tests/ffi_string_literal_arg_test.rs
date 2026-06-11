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

// TDD Test: String literals passed to extern fn should auto-convert to FfiString
//
// When a Windjammer extern fn declares a parameter as `string`,
// the generated Rust signature uses `FfiString`. String literal arguments
// should be automatically wrapped with `string_to_ffi()`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_string_literal_to_extern_fn_is_wrapped() {
    let code = r#"
extern fn save_file(path: String, data: i32)

pub fn do_save() {
    save_file("fixture_output.txt", 42)
}
"#;
    let rust_code = test_utils::compile_single(code);

    assert!(
        rust_code.contains("string_to_ffi") || rust_code.contains("FfiString"),
        "String literal to extern fn should be wrapped with FfiString conversion.\nGenerated:\n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("save_file(\"fixture_output.txt\""),
        "Raw string literal should NOT be passed directly to extern fn.\nGenerated:\n{}",
        rust_code
    );
}
