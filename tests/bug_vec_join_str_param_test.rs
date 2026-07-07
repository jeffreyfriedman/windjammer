#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

// Bug: Vec::join() generates .to_string() on separator, producing E0308
//
// In Rust, `Vec<String>::join(separator)` takes `&str`, not `String`.
// The codegen was adding `.to_string()` to string literal separators,
// producing `items.join("\n".to_string())` instead of `items.join("\n")`.
// This causes `expected &str, found String` errors.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn vec_join_string_literal_separator_no_to_string() {
    let source = r#"
pub fn join_items() -> string {
    let mut items = Vec::new()
    items.push("hello")
    items.push("world")
    items.join(", ")
}
"#;
    let (generated, success) = test_utils::compile_single_check(source);
    assert!(success, "compilation should succeed");
    assert!(
        !generated.contains(r#".join(", ".to_string())"#),
        "join() separator should NOT have .to_string().\nGenerated:\n{}",
        generated
    );
}

#[test]
fn vec_join_empty_separator_no_to_string() {
    let source = r#"
pub fn concat_items() -> string {
    let mut items = Vec::new()
    items.push("a")
    items.push("b")
    items.join("")
}
"#;
    let (generated, success) = test_utils::compile_single_check(source);
    assert!(success, "compilation should succeed");
    assert!(
        !generated.contains(r#".join("".to_string())"#),
        "join(\"\") should NOT have .to_string().\nGenerated:\n{}",
        generated
    );
}

#[test]
fn vec_join_newline_separator_no_to_string() {
    let source = r#"
pub fn render_lines() -> string {
    let mut lines = Vec::new()
    lines.push("line1")
    lines.push("line2")
    lines.join("\n")
}
"#;
    let (generated, success) = test_utils::compile_single_check(source);
    assert!(success, "compilation should succeed");
    assert!(
        !generated.contains(r#".join("\n".to_string())"#),
        "join(\"\\n\") should NOT have .to_string().\nGenerated:\n{}",
        generated
    );
}

#[test]
fn vec_join_compiles_with_rustc() {
    let source = r#"
pub fn render_html() -> string {
    let mut items = Vec::new()
    items.push("<li>one</li>")
    items.push("<li>two</li>")
    items.join("\n")
}
"#;
    let generated = test_utils::compile_single(source);
    test_utils::verify_rust_compiles(&generated).expect("Generated Rust should compile");
}
