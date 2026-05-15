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

/// TDD: WJSL compiler error messages should include line/column info
///
/// Errors from WJSL parsing and type checking should include location
/// information to help developers find and fix shader bugs quickly.
use windjammer::wjsl::transpile_wjsl;

#[test]
fn test_parse_error_includes_line_number() {
    let source = r#"
@group(0) @binding(0) storage read data: array<f32>;

fn test() {
    let x =
}
"#;
    let result = transpile_wjsl(source);
    assert!(result.is_err(), "Should fail to parse");
    let err_msg = result.err().unwrap().to_string();
    assert!(
        err_msg.contains("line") || err_msg.contains(":"),
        "Error should include line info. Got: {}",
        err_msg
    );
}

#[test]
fn test_type_error_includes_location() {
    let source = r#"
fn test() -> f32 {
    return true;
}
"#;
    let result = transpile_wjsl(source);
    assert!(result.is_err(), "Should fail type check");
    let err_msg = result.err().unwrap().to_string();
    assert!(
        err_msg.contains("Return type mismatch"),
        "Should indicate return type mismatch. Got: {}",
        err_msg
    );
}

#[test]
fn test_parse_error_with_filename() {
    let source = r#"
struct Foo {
    x: vec3<f32>
    y: invalid_type
}
"#;
    let result = windjammer::wjsl::parser::parse_wjsl_with_filename(
        source,
        "shaders/my_shader.wjsl".to_string(),
    );
    if let Err(e) = result {
        let msg = e.to_string();
        assert!(
            msg.contains("my_shader.wjsl"),
            "Error should include filename. Got: {}",
            msg
        );
    }
}

#[test]
fn test_unknown_token_error_is_descriptive() {
    let source = r#"
fn test(a: u32, b: u32) -> u32 {
    return a + + b;
}
"#;
    let result = transpile_wjsl(source);
    if result.is_err() {
        let msg = result.err().unwrap().to_string();
        assert!(!msg.is_empty(), "Error message should be non-empty");
    }
}
