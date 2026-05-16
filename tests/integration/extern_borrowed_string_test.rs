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

/// TDD Test: Extern function calls with borrowed string arguments
///
/// Bug: When calling an extern function that takes `string` (→ String in Rust),
/// passing a borrowed string param (e.g. `data: string` inferred as &str) generates
/// `string_to_ffi(data)` but string_to_ffi expects String, not &str.
///   error[E0308]: mismatched types -- expected `String`, found `&str`
///
/// Root Cause: Codegen checks infer_expression_type which returns the declared
/// param type (Type::String), not the actual Rust type (&str when Borrowed).
/// So we skip .to_string() incorrectly.
///
/// Fix: Always use .to_string() for string args to extern functions.
/// Works for both &str and String (String::to_string() returns clone).
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_extern_borrowed_string_param() {
    // verify_checksum(data, expected) - data is inferred as &str (Borrowed)
    // save_checksum_hash(data) must pass String to string_to_ffi
    let source = r#"
extern fn compute_hash(data: string) -> string

pub fn verify(data: string, expected: string) -> bool {
    let actual = compute_hash(data)
    actual == expected
}
"#;

    let result = test_utils::compile_single_result(source);
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    let rust = result.unwrap();

    // Borrowed param data → &str in Rust → must convert to String for string_to_ffi
    assert!(
        rust.contains("string_to_ffi(data.to_string())"),
        "Borrowed string param should get .to_string() before string_to_ffi.\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_extern_owned_string_param() {
    // When param is owned (e.g. consumed), .to_string() still works (String::to_string = clone)
    let source = r#"
extern fn compute_hash(data: string) -> string

pub fn verify(data: string) -> string {
    compute_hash(data)
}
"#;

    let result = test_utils::compile_single_result(source);
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    let rust = result.unwrap();

    // Should use string_to_ffi - with .to_string() for safety (works for both &str and String)
    assert!(
        rust.contains("string_to_ffi("),
        "Should wrap extern string arg with string_to_ffi.\nGenerated:\n{}",
        rust
    );
    assert!(
        rust.contains(".to_string()"),
        "Should use .to_string() for extern string params (handles both &str and String).\nGenerated:\n{}",
        rust
    );
}
