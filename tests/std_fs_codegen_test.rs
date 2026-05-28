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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_std_fs_qualified_path_compiles() {
    // Qualified std::fs::read_to_string should pass through as valid Rust
    let output = test_utils::compile_single(
        r#"
fn read_file(path: String) -> String {
    let content = std::fs::read_to_string(path)
    content
}
"#,
    );
    assert!(
        output.contains("std::fs::read_to_string"),
        "Qualified std::fs::read_to_string should pass through. Got:\n{}",
        output
    );
}

#[test]
fn test_use_std_fs_generates_import() {
    // `use std::fs` should generate a Rust import so unqualified fs::read_to_string works
    let output = test_utils::compile_single(
        r#"
use std::fs

fn read_file(path: String) -> String {
    let content = fs::read_to_string(path)
    content
}
"#,
    );
    // Should generate `use std::fs;` or similar
    assert!(
        output.contains("use std::fs"),
        "use std::fs should generate an import. Got:\n{}",
        output
    );
}

#[test]
fn test_std_fs_write_qualified() {
    let output = test_utils::compile_single(
        r#"
fn write_file(path: String, data: String) {
    std::fs::write(path, data)
}
"#,
    );
    assert!(
        output.contains("std::fs::write"),
        "std::fs::write should pass through. Got:\n{}",
        output
    );
}
