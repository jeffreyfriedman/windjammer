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

/// TDD Test: Function Call Return Type Inference
///
/// Bug: infer_expression_type() should return the function's return type
/// Pattern: result = result + func() where func() -> String
/// Expected: infer_expression_type(func()) should return Some(Type::String)
///
/// This is critical for compound assignment optimization:
/// - If right side is String, can't use += (needs =)
/// - If right side is &str, can use += (efficient)
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_function_call_returns_string() {
    let source = r#"
pub fn greet(name: string) -> string {
    format!("Hello, {}!", name)
}

pub fn concatenate_greetings() -> string {
    let mut result = ""
    result = result + greet("Alice")
    result = result + greet("Bob")
    result
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // TDD VERIFICATION: Should NOT use += because greet() returns String
    // String += String doesn't work in Rust
    assert!(
        !output.contains("result += greet"),
        "Should use = not += when function returns String: {}",
        output
    );

    // Should use regular assignment with & prefix for borrowing
    assert!(
        output.contains("result = result + &greet") || output.contains("result.push_str"),
        "Should use = with & prefix or push_str for String concatenation: {}",
        output
    );
}

#[test]
fn test_method_call_returns_string() {
    let source = r#"
struct Formatter {
    prefix: string,
}

impl Formatter {
    pub fn format(self, text: string) -> string {
        format!("{}: {}", self.prefix, text)
    }
}

pub fn build_output(fmt: Formatter) -> string {
    let mut result = ""
    result = result + fmt.format("first")
    result = result + fmt.format("second")
    result
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // Should NOT use += because format() returns String
    // Should use = with & prefix for borrowing
    assert!(
        !output.contains("result += "),
        "Should use = not += when method returns String: {}",
        output
    );
    assert!(
        output.contains("result = result + &"),
        "Should add & prefix for String borrowing: {}",
        output
    );
}

#[test]
fn test_format_macro_returns_string() {
    let source = r#"
pub fn build_message(name: string, age: i32) -> string {
    let mut result = ""
    result = result + format!("Name: {}", name)
    result = result + format!("Age: {}", age)
    result
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // Should NOT use += because format! returns String
    // Should use = with & prefix for borrowing
    assert!(
        !output.contains("result += format"),
        "Should use = not += when format! returns String: {}",
        output
    );
    assert!(
        output.contains("result = result + &format"),
        "Should add & prefix for String borrowing: {}",
        output
    );
}

// Helper function
