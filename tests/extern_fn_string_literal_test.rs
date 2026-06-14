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

/// TDD Test: Extern function calls with string literal arguments expecting String
///
/// Bug: When calling an extern function that takes `string` (→ String in Rust),
/// passing a string literal like `"hello"` generates `"hello"` (&str) instead
/// of `"hello".to_string()` (String). This causes a type mismatch error:
///   error[E0308]: mismatched types -- expected `String`, found `&str`
///
/// Root Cause: The analyzer infers `Borrowed` ownership for extern function
/// parameters (since the body is empty), but the string-literal-to-String
/// conversion only fires for `Owned` ownership. For extern functions, the
/// parameter type (String) should take precedence over inferred ownership.
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_extern_fn_string_literal_gets_to_string() {
    let code = r#"
extern fn load_shader(path: String) -> u32

fn main() {
    let id = load_shader("shaders/test.wgsl")
}
"#;

    let generated = test_utils::compile_single_result(code).expect("Compilation should succeed");

    assert!(
        generated.contains(r#""shaders/test.wgsl".to_string()"#),
        "String literal passed to extern fn expecting String should get .to_string().\nGenerated code:\n{}",
        generated
    );
}

#[test]
fn test_extern_fn_string_literal_multiple_params() {
    let code = r#"
extern fn create_window(title: String, width: u32, height: u32)

fn main() {
    create_window("My Window", 800, 600)
}
"#;

    let generated = test_utils::compile_single_result(code).expect("Compilation should succeed");

    assert!(
        generated.contains(r#""My Window".to_string()"#),
        "String literal in first param should get .to_string().\nGenerated code:\n{}",
        generated
    );
}

#[test]
fn test_extern_fn_string_literal_variable_still_works() {
    let code = r#"
extern fn load_file(path: String) -> u32

fn main() {
    let p = "test.txt"
    let id = load_file(p)
}
"#;

    let generated = test_utils::compile_single_result(code).expect("Compilation should succeed");

    assert!(
        generated.contains("load_file("),
        "Should generate a call to load_file.\nGenerated code:\n{}",
        generated
    );
}
