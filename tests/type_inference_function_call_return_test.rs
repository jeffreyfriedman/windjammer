/// TDD Test: Function Call Return Type Inference
///
/// Bug: infer_expression_type() should return the function's return type
/// Pattern: result = result + func() where func() -> String
/// Expected: infer_expression_type(func()) should return Some(Type::String)
///
/// This is critical for compound assignment optimization:
/// - If right side is String, can't use += (needs =)
/// - If right side is &str, can use += (efficient)

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

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

    let output = compile_and_get_rust(source);
    
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

    let output = compile_and_get_rust(source);
    
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

    let output = compile_and_get_rust(source);
    
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
fn compile_and_get_rust(source: &str) -> String {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_dir = format!("/tmp/func_return_test_{}_{}", std::process::id(), counter);
    
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
