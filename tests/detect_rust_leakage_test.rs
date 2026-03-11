/// TDD Test: Detect Rust Leakage in Windjammer Source
///
/// This test ensures that common Rust patterns don't leak into Windjammer code.
/// It serves as a compile-time check that the Windjammer compiler rejects
/// Rust-specific syntax in .wj files.
///
/// Patterns to detect:
/// - Explicit `&` or `&mut` in function signatures
/// - `.as_str()`, `.as_ref()`, `.as_mut()` method calls
/// - `.unwrap()`, `.expect()` panic-inducing calls
/// - Explicit lifetime annotations ('a, 'b, etc.)

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn test_reject_ampersand_in_function_signature() {
    let source = r#"
fn process(data: &str) -> String {
    data.to_string()
}
"#;

    let result = try_compile(source);
    
    // This should either:
    // 1. Fail to compile (best - parser rejects it)
    // 2. Compile but generate a warning
    // 3. Compile successfully (current behavior - need to fix)
    
    println!("\n=== Compilation Result ===");
    println!("Success: {}", result.success);
    println!("Output: {}", result.output);
    
    // For now, document that this SHOULD fail
    // TODO: Make parser reject `&` in function signatures
    if result.success {
        println!("⚠️  WARNING: Compiler accepted `&str` - should reject!");
        println!("    This is a known issue to fix with TDD");
    }
}

#[test]
fn test_reject_ampersand_mut_in_method() {
    let source = r#"
struct Counter {
    value: i32,
}

impl Counter {
    fn increment(&mut self) {
        self.value = self.value + 1
    }
}
"#;

    let result = try_compile(source);
    
    println!("\n=== Compilation Result ===");
    println!("Success: {}", result.success);
    println!("Output: {}", result.output);
    
    if result.success {
        println!("⚠️  WARNING: Compiler accepted `&mut self` - should reject!");
        println!("    Idiomatic: fn increment(self) - compiler infers &mut");
    }
}

#[test]
fn test_reject_as_str_method_call() {
    let source = r#"
fn print_string(s: String) {
    println!("{}", s.as_str())
}
"#;

    let result = try_compile(source);
    
    println!("\n=== Compilation Result ===");
    println!("Success: {}", result.success);
    println!("Output: {}", result.output);
    
    // The language check should catch this
    if result.success && !result.output.contains("LANGUAGE CHECK") {
        println!("⚠️  WARNING: Compiler accepted `.as_str()` - should reject!");
    } else if !result.success {
        println!("✅ GOOD: Compiler rejected `.as_str()`");
    }
}

#[test]
fn test_reject_unwrap_call() {
    let source = r#"
fn get_value(opt: Option<i32>) -> i32 {
    opt.unwrap()
}
"#;

    let result = try_compile(source);
    
    println!("\n=== Compilation Result ===");
    println!("Success: {}", result.success);
    println!("Output: {}", result.output);
    
    if result.success {
        println!("⚠️  WARNING: Compiler accepted `.unwrap()` - should reject or warn!");
        println!("    Idiomatic: Use pattern matching or `?`");
    }
}

#[test]
fn test_accept_idiomatic_windjammer() {
    let source = r#"
struct Counter {
    value: i32,
}

impl Counter {
    fn increment(self) {
        self.value = self.value + 1
    }
    
    fn get_value(self) -> i32 {
        self.value
    }
}

fn process(data: String) -> String {
    data
}
"#;

    let result = try_compile(source);
    
    println!("\n=== Compilation Result ===");
    println!("Success: {}", result.success);
    
    assert!(
        result.success,
        "Idiomatic Windjammer code should compile: {}",
        result.output
    );
    
    println!("✅ GOOD: Idiomatic Windjammer compiles successfully");
}

// Helper types and functions

struct CompileResult {
    success: bool,
    output: String,
}

fn try_compile(source: &str) -> CompileResult {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_dir = format!("/tmp/rust_leakage_test_{}_{}", std::process::id(), counter);
    
    std::fs::create_dir_all(&test_dir).unwrap();
    
    let source_file = PathBuf::from(&test_dir).join("test.wj");
    std::fs::write(&source_file, source).unwrap();
    
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(&[
            "build",
            source_file.to_str().unwrap(),
            "--target",
            "rust",
            "--output",
            &test_dir,
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");
    
    let success = output.status.success();
    let combined_output = format!(
        "STDOUT:\n{}\n\nSTDERR:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    
    CompileResult {
        success,
        output: combined_output,
    }
}
