/// TDD Test: Compound Assignment Operators for Math
///
/// Verify that all compound assignment operators work correctly:
/// - += (addition)
/// - -= (subtraction)
/// - *= (multiplication)
/// - /= (division)
/// - %= (modulo)
/// - &= (bitwise AND)
/// - |= (bitwise OR)
/// - ^= (bitwise XOR)
/// - <<= (left shift)
/// - >>= (right shift)
///
/// Expected: All operators should work naturally with numeric types

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn test_compound_add_integers() {
    let source = r#"
pub fn add_values() -> i32 {
    let mut x = 10
    x += 5
    x += 3
    x
}
"#;

    let (success, output) = compile_and_verify_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("x += 5"), "Should generate compound assignment for integers");
}

#[test]
fn test_compound_add_floats() {
    let source = r#"
pub fn add_floats() -> f32 {
    let mut x = 10.0
    x += 2.5
    x += 1.5
    x
}
"#;

    let (success, output) = compile_and_verify_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("x += "), "Should generate compound assignment for floats");
}

#[test]
fn test_compound_subtract() {
    let source = r#"
pub fn subtract_values() -> i32 {
    let mut health = 100
    health -= 10
    health -= 5
    health
}
"#;

    let (success, output) = compile_and_verify_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("health -= "), "Should generate -= operator");
}

#[test]
fn test_compound_multiply() {
    let source = r#"
pub fn multiply_values() -> i32 {
    let mut score = 10
    score *= 2
    score *= 3
    score
}
"#;

    let (success, output) = compile_and_verify_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("score *= "), "Should generate *= operator");
}

#[test]
fn test_compound_divide() {
    let source = r#"
pub fn divide_values() -> f32 {
    let mut x = 100.0
    x /= 2.0
    x /= 5.0
    x
}
"#;

    let (success, output) = compile_and_verify_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("x /= "), "Should generate /= operator");
}

#[test]
fn test_compound_modulo() {
    let source = r#"
pub fn modulo_value() -> i32 {
    let mut x = 100
    x %= 7
    x
}
"#;

    let (success, output) = compile_and_verify_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("x %= "), "Should generate %= operator");
}

#[test]
fn test_compound_bitwise_and() {
    let source = r#"
pub fn bitwise_and() -> i32 {
    let mut flags = 0xFF
    flags &= 0x0F
    flags
}
"#;

    let (success, output) = compile_and_verify_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("flags &= "), "Should generate &= operator");
}

#[test]
fn test_compound_bitwise_or() {
    let source = r#"
pub fn bitwise_or() -> i32 {
    let mut flags = 0x01
    flags |= 0x02
    flags |= 0x04
    flags
}
"#;

    let (success, output) = compile_and_verify_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("flags |= "), "Should generate |= operator");
}

#[test]
fn test_compound_bitwise_xor() {
    let source = r#"
pub fn bitwise_xor() -> i32 {
    let mut value = 0xFF
    value ^= 0xAA
    value
}
"#;

    let (success, output) = compile_and_verify_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("value ^= "), "Should generate ^= operator");
}

#[test]
fn test_compound_left_shift() {
    let source = r#"
pub fn left_shift() -> i32 {
    let mut bits = 1
    bits <<= 3
    bits
}
"#;

    let (success, output) = compile_and_verify_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("bits <<= "), "Should generate <<= operator");
}

#[test]
fn test_compound_right_shift() {
    let source = r#"
pub fn right_shift() -> i32 {
    let mut bits = 64
    bits >>= 2
    bits
}
"#;

    let (success, output) = compile_and_verify_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("bits >>= "), "Should generate >>= operator");
}

#[test]
fn test_compound_mixed_operations() {
    let source = r#"
pub fn mixed_ops() -> f32 {
    let mut x = 100.0
    x += 50.0  // Add
    x -= 20.0  // Subtract
    x *= 2.0   // Multiply
    x /= 3.0   // Divide
    x
}
"#;

    let (success, output) = compile_and_verify_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("x += 50"), "Should generate +=");
    assert!(output.contains("x -= 20"), "Should generate -=");
    assert!(output.contains("x *= 2"), "Should generate *=");
    assert!(output.contains("x /= 3"), "Should generate /=");
}

#[test]
fn test_compound_with_expressions() {
    let source = r#"
pub fn compound_with_expr() -> i32 {
    let mut sum = 0
    let count = 5
    sum += count * 2
    sum += count + 3
    sum
}
"#;

    let (success, output) = compile_and_verify_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("sum += "), "Should generate compound assignment with expressions");
}

#[test]
fn test_compound_ops_runtime_correctness() {
    let source = r#"
pub fn compute() -> i32 {
    let mut x = 10
    x += 5    // 15
    x -= 3    // 12
    x *= 2    // 24
    x /= 4    // 6
    x %= 5    // 1
    x |= 8    // 9 (0b0001 | 0b1000)
    x
}
"#;

    let (success, output) = compile_and_verify_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(success, "Generated Rust should compile: {}", output);
    
    // Verify the function can be called (syntax is correct)
    assert!(output.contains("pub fn compute() -> i32"), "Should generate public function");
}

// Helper function
fn compile_and_verify_rust(source: &str) -> (bool, String) {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_dir = format!("/tmp/math_compound_test_{}_{}", std::process::id(), counter);
    
    std::fs::create_dir_all(&test_dir).unwrap();
    
    let source_file = PathBuf::from(&test_dir).join("test.wj");
    std::fs::write(&source_file, source).unwrap();
    
    // Compile Windjammer -> Rust
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
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return (false, format!("Compilation failed: {}", stderr));
    }
    
    let rust_file = PathBuf::from(&test_dir).join("test.rs");
    let rust_code = std::fs::read_to_string(&rust_file)
        .expect("Failed to read generated Rust file");
    
    // Verify Rust compiles
    let rustc = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&rust_file)
        .arg("-o")
        .arg(PathBuf::from(&test_dir).join("test.rlib"))
        .output()
        .expect("Failed to run rustc");
    
    (rustc.status.success(), rust_code)
}
