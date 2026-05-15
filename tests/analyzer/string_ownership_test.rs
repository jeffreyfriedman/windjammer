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

/// TDD Test: String Ownership in Method Calls
///
/// Bug: Compiler adds `&` when passing String to method expecting owned String
/// Expected: String variables should be passed as-is (moved)
/// Actual: Compiler adds unnecessary `&String`
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_passed_owned_to_method() {
    let source = r#"
struct Renderer {}

impl Renderer {
    fn draw_text(self, text: string) {
        println!("{}", text)
    }
}

fn main() {
    let renderer = Renderer{}
    let message = "Hello".to_string()
    renderer.draw_text(message)  // Should pass owned, not &message
}
"#;

    let rust_code = test_utils::compile_single_result(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // NEW DESIGN: Read-only string parameters infer to &str
    // text: string (read-only) → infers to text: &str
    // Call site: owned String is borrowed with &
    assert!(
        rust_code.contains("text: &str"),
        "Read-only string parameter should infer to &str.\nGenerated:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("renderer.draw_text(&message)"),
        "Owned String should be borrowed when passed to &str parameter.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_literal_converted_to_string() {
    let source = r#"
struct Renderer {}

impl Renderer {
    fn draw_text(&self, text: string) {
        println!("{}", text)
    }
}

fn main() {
    let renderer = Renderer{}
    renderer.draw_text("Hello World")  // Should convert &str to String
}
"#;

    let rust_code = test_utils::compile_single_result(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // NEW DESIGN: Read-only string parameters infer to &str
    // String literals are already &str, so they're passed directly
    assert!(
        rust_code.contains("text: &str"),
        "Read-only string parameter should infer to &str.\nGenerated:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains(r#"draw_text("Hello World")"#),
        "String literal should be passed directly to &str parameter.\nGenerated:\n{}",
        rust_code
    );
}
