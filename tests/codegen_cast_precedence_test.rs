// TDD TEST: Cast expressions must be wrapped in parentheses before operators/methods
//
// BUG: The compiler generates `x as T.method()` which Rust parses as `x as (T.method())`
//      instead of `(x as T).method()`
//
// ROOT CAUSE: Code generator doesn't add parentheses around cast expressions
//
// FIX: Wrap cast expressions in parentheses when followed by operators or method calls

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_cast_followed_by_method() {
    let wj_code = r#"
pub fn test_cast_method() {
    let x = 10
    let result = (x as f32).sqrt()
}

fn main() {
    test_cast_method()
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path();
    let wj_file = output_dir.join("test.wj");
    fs::write(&wj_file, wj_code).unwrap();

    // Compile
    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_file)
        .arg("--output")
        .arg(output_dir)
        .output()
        .expect("Failed to run wj");

    assert!(
        result.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    // Read generated Rust
    let rust_file = output_dir.join("test.rs");
    let rust_code = fs::read_to_string(rust_file).unwrap();

    // Should have parentheses around cast before method call
    assert!(
        rust_code.contains("(x as f32).sqrt()") || rust_code.contains("(x.clone() as f32).sqrt()"),
        "Cast should be wrapped in parentheses before method call. Generated:\n{}",
        rust_code
    );

    println!("✅ Generated Rust has correct cast precedence");
}

#[test]
fn test_cast_followed_by_comparison() {
    let wj_code = r#"
pub fn test_cast_comparison(i: i64, len: i64) -> bool {
    (i as usize) < (len as usize)
}

fn main() {
    let result = test_cast_comparison(5, 10)
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path();
    let wj_file = output_dir.join("test.wj");
    fs::write(&wj_file, wj_code).unwrap();

    // Compile
    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_file)
        .arg("--output")
        .arg(output_dir)
        .output()
        .expect("Failed to run wj");

    assert!(
        result.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    // Read generated Rust
    let rust_file = output_dir.join("test.rs");
    let rust_code = fs::read_to_string(rust_file).unwrap();

    // Should have parentheses around casts in comparison
    assert!(
        rust_code.contains("(i as usize) <") || rust_code.contains("(i.clone() as usize) <"),
        "Cast should be wrapped in parentheses in comparison. Generated:\n{}",
        rust_code
    );

    println!("✅ Generated Rust has correct cast precedence in comparisons");
}
