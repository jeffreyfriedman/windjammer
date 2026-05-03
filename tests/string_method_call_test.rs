/// TDD Test: String Method Call Ownership Inference
///
/// Bug: String.push_str(String) should auto-convert to &str
/// Pattern: html.push_str(self.class.as_str()) - .as_str() shouldn't be needed
/// Root Cause: Compiler not inferring &str conversion for push_str parameter
/// Expected: html.push_str(self.class) should compile (auto-convert)
#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_push_str_with_string_field() {
    let source = r#"
struct Menu {
    class: String,
}

impl Menu {
    pub fn render() -> String {
        let mut html = String::new()
        html.push_str("<div class=\"")
        html.push_str(self.class)
        html.push_str("\">")
        html
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // Should compile without .as_str()
    assert!(
        output.contains("html.push_str"),
        "Should contain push_str calls: {}",
        output
    );

    // Should NOT contain .as_str() anywhere
    assert!(
        !output.contains(".as_str()"),
        "Should not need .as_str() - compiler handles it: {}",
        output
    );
}

#[test]
fn test_push_str_with_string_variable() {
    let source = r#"
pub fn concat(a: String, b: String) -> String {
    let mut result = String::new()
    result.push_str(a)
    result.push_str(b)
    result
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(
        !output.contains(".as_str()"),
        "Should not need .as_str(): {}",
        output
    );
}

#[test]
fn test_push_str_with_format_result() {
    let source = r#"
pub fn format_message(name: String, age: i32) -> String {
    let mut result = String::new()
    result.push_str(format!("Name: {}", name))
    result.push_str(format!(", Age: {}", age))
    result
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(
        !output.contains(".as_str()"),
        "Should not need .as_str() on format! results: {}",
        output
    );
}

// Helper function to compile Windjammer code and return generated Rust
