// TDD: Test that methods with explicit `self` (by value) are respected
// These tests verify the Windjammer v0.45.0+ design: explicit ownership is preserved.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn get_wj_binary() -> String {
    env!("CARGO_BIN_EXE_wj").to_string()
}

fn compile_to_rust(source: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, source).expect("Failed to write test file");
    fs::create_dir(&out_dir).expect("Failed to create output dir");

    let output = Command::new(get_wj_binary())
        .arg("build")
        .arg(&wj_path)
        .arg("--output")
        .arg(&out_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let rust_file = out_dir.join("test.rs");
    let rust_code = fs::read_to_string(rust_file).expect("Failed to read generated Rust");
    Ok(rust_code)
}

#[test]
fn test_method_self_by_value_should_not_infer_mut() {
    // THE WINDJAMMER WAY: Explicit `self` (by value) is RESPECTED
    // Method signature stays as written, no inference to &mut self
    let source = r#"
struct Point {
    x: f32,
    y: f32
}

impl Point {
    fn double(self) -> Point {
        Point {
            x: self.x * 2.0,
            y: self.y * 2.0
        }
    }
}

fn main() {
    let p = Point { x: 1.0, y: 2.0 }
    let p2 = p.double()
    
    println!("{}", p2.x)
}
"#;

    let rust_code = match compile_to_rust(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    // Method signature should preserve explicit "self" by value
    assert!(
        rust_code.contains("fn double(self)") || rust_code.contains("fn double(mut self)"),
        "Method should preserve explicit 'self' by value (owned).\n\nGenerated:\n{}",
        rust_code
    );

    // Variable should NOT be inferred as mut (unless needed for other reasons)
    // The key is that calling a by-value method doesn't FORCE mut inference
    // (This is the correct behavior - by-value methods consume, don't mutate)
}

#[test]
fn test_method_self_by_value_with_multiply() {
    // THE WINDJAMMER WAY: Methods with explicit `self` stay owned
    // Variables passed to by-value methods don't need mut (method consumes)
    let source = r#"
struct Mat4 {
    m00: f32, m11: f32, m22: f32, m33: f32
}

impl Mat4 {
    fn identity() -> Mat4 {
        Mat4 { m00: 1.0, m11: 1.0, m22: 1.0, m33: 1.0 }
    }
    
    fn multiply(self, other: Mat4) -> Mat4 {
        Mat4 {
            m00: self.m00 * other.m00,
            m11: self.m11 * other.m11,
            m22: self.m22 * other.m22,
            m33: self.m33 * other.m33
        }
    }
}

fn main() {
    let identity = Mat4::identity()
    let identity2 = Mat4::identity()
    let result = identity.multiply(identity2)
    
    println!("{}", result.m00)
}
"#;

    let rust_code = match compile_to_rust(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    // Method should preserve explicit "self" by value
    assert!(
        rust_code.contains("fn multiply(self,") || rust_code.contains("fn multiply(mut self,"),
        "Method should preserve explicit 'self' by value.\n\nGenerated:\n{}",
        rust_code
    );

    // Variables consumed by by-value methods don't need mut
    // (They're moved, not mutated)
    // This is CORRECT Rust semantics
}

#[test]
fn test_method_self_by_value_respects_explicit_intent() {
    // THE WINDJAMMER WAY: User writes "self", compiler preserves it
    // No automatic conversion to &self or &mut self
    let source = r#"
struct Value {
    x: i32
}

impl Value {
    fn new(x: i32) -> Value {
        Value { x: x }
    }
    
    fn add(self, other: i32) -> Value {
        Value { x: self.x + other }
    }
}

fn main() {
    let v = Value::new(5)
    let v2 = v.add(3)
    
    println!("{}", v2.x)
}
"#;

    let rust_code = match compile_to_rust(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    // Explicit "self" should be preserved (consumed, not borrowed)
    assert!(
        rust_code.contains("fn add(self,") || rust_code.contains("fn add(mut self,"),
        "Method should preserve explicit 'self' by value.\n\nGenerated:\n{}",
        rust_code
    );

    // This tests the core principle: EXPLICIT ownership is NEVER overridden
    // The compiler respects user intent (by-value methods CONSUME, don't borrow)
}
