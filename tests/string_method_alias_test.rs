#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! WJ-LANG-04: `.string()` is the idiomatic Windjammer alias for value-to-string conversion.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_string_method_on_int_lowers_to_to_string() {
    let code = r#"
    pub fn format_count(n: int) -> string {
        n.string()
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");
    assert!(
        generated.contains(".to_string()"),
        "`.string()` should lower to `.to_string()` in Rust. Got:\n{}",
        generated
    );
    assert!(
        !generated.contains(".string()"),
        "Generated Rust should not contain `.string()`. Got:\n{}",
        generated
    );
}

#[test]
fn test_string_method_on_bool() {
    let code = r#"
    pub fn format_flag(flag: bool) -> string {
        flag.string()
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");
    assert!(
        generated.contains(".to_string()"),
        "bool.string() should lower to .to_string(). Got:\n{}",
        generated
    );
}
