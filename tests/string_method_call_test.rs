/// TDD Test: String Method Call Ownership Inference
///
/// Bug: String.push_str(String) should auto-convert to &str
/// Pattern: html.push_str(self.class.as_str()) - .as_str() shouldn't be needed
/// Root Cause: Compiler not inferring &str conversion for push_str parameter
/// Expected: html.push_str(self.class) should compile (auto-convert)

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

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

    let output = compile_and_get_rust(source);
    
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

    let output = compile_and_get_rust(source);
    
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

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(
        !output.contains(".as_str()"),
        "Should not need .as_str() on format! results: {}",
        output
    );
}

// Helper function to compile Windjammer code and return generated Rust
fn compile_and_get_rust(source: &str) -> String {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_dir = format!("/tmp/string_method_test_{}_{}", std::process::id(), counter);
    
    std::fs::create_dir_all(&test_dir).unwrap();
    
    let source_file = PathBuf::from(&test_dir).join("test.wj");
    std::fs::write(&source_file, source).unwrap();
    
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(&[
            "build",
            source_file.to_str().unwrap(),
            "--target", "rust",
            "--output", &test_dir,
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");
    
    assert!(
        output.status.success(),
        "Compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    
    let rust_file = PathBuf::from(&test_dir).join("test.rs");
    std::fs::read_to_string(&rust_file)
        .expect("Failed to read generated Rust file")
}
