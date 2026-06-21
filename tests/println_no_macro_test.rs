#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! TDD: println without ! macro syntax — runtime io when stdlib is linked.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_println_simple_string_standalone() {
    let code = r#"
    pub fn greet() {
        println("Hello, World!")
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    assert!(
        generated.contains("println!(\"Hello, World!\")"),
        "Standalone build should use println! macro. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_println_with_stdlib_uses_runtime_io() {
    let code = r#"
    use std::http::*

    pub fn log_status() {
        println("Server ready")
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    assert!(
        generated.contains("windjammer_runtime::io::println(\"Server ready\")"),
        "Stdlib-linked build should use runtime io. Generated:\n{}",
        generated
    );
    assert!(
        !generated.contains("println!("),
        "Must not leak Rust println! when runtime is linked. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_println_with_format_standalone() {
    let code = r#"
    pub fn log_value(x: int) {
        println("Value: {}", x)
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    assert!(
        generated.contains("println!(\"Value: {}\", x)"),
        "Should generate println! with format args. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_println_with_format_runtime() {
    let code = r#"
    use std::env

    pub fn log_value(x: int) {
        println("Value: {}", x)
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    assert!(
        generated.contains("windjammer_runtime::io::println(&format!(\"Value: {}\", x))"),
        "Stdlib build should format via runtime io. Generated:\n{}",
        generated
    );
}
