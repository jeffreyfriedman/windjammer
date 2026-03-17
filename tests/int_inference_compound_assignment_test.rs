//! TDD Test: Compound assignment type inference
//!
//! Bug: `count += 1` where count is u32 generates 1_i32 instead of 1_u32
//! Root Cause: IntInference doesn't constrain RHS literal to match LHS type for compound assignments
//!
//! Tests:
//! - u32 += literal → literal must be u32
//! - i64 += literal → literal must be i64
//! - f32 += literal → (float inference, not int - skip)
//! - vec[i] += literal where Vec<u32> → literal must be u32

use std::fs;
use std::process::Command;

fn compile_and_get_rust(wj_source: &str, test_name: &str) -> (bool, String, String) {
    let output_dir = format!("/tmp/wj_compound_assign_{}", test_name);
    fs::create_dir_all(&output_dir).unwrap();
    fs::write(format!("{}/test.wj", output_dir), wj_source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            "--target",
            "rust",
            "--no-cargo",
            &format!("{}/test.wj", output_dir),
            "--output",
            &output_dir,
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run wj");

    let _stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let rust_code = fs::read_to_string(format!("{}/test.rs", output_dir))
        .unwrap_or_else(|_| String::from("(file not generated)"));

    (output.status.success(), rust_code, stderr)
}

fn rustc_compile(rust_code: &str) -> (bool, String) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("rustc_compound_{}", timestamp));
    fs::create_dir_all(&temp_dir).unwrap();
    fs::write(temp_dir.join("test.rs"), rust_code).unwrap();

    let output = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(temp_dir.join("test.rs"))
        .arg("--out-dir")
        .arg(&temp_dir)
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let _ = fs::remove_dir_all(&temp_dir);

    (output.status.success(), stderr)
}

#[test]
fn test_u32_compound_add_assign() {
    // Explicit type ensures var_types has count before processing count += 1
    let source = r#"
fn get_count() -> u32 {
    let mut count: u32 = 0
    count += 1
    return count
}

fn main() {
    let _x = get_count()
}
"#;

    let (wj_ok, rust_code, _stderr) = compile_and_get_rust(source, "u32_add");

    assert!(wj_ok, "Windjammer should compile");

    assert!(
        rust_code.contains("1_u32"),
        "count += 1 where count: u32 should generate 1_u32, got:\n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("1_i32"),
        "Should NOT generate 1_i32 for u32 compound assign, got:\n{}",
        rust_code
    );

    let (rustc_ok, rustc_stderr) = rustc_compile(&rust_code);
    assert!(
        rustc_ok,
        "Generated Rust should compile with rustc:\n{}",
        rustc_stderr
    );
}

#[test]
fn test_i64_compound_add_assign() {
    // Use explicit type so var_types has total before we process total += 42
    let source = r#"
fn get_total() -> int {
    let mut total: int = 0
    total += 42
    return total
}

fn main() {
    let _x = get_total()
}
"#;

    let (wj_ok, rust_code, _stderr) = compile_and_get_rust(source, "i64_add");

    assert!(wj_ok, "Windjammer should compile");

    assert!(
        rust_code.contains("42_i64"),
        "total += 42 where total: int should generate 42_i64, got:\n{}",
        rust_code
    );

    let (rustc_ok, rustc_stderr) = rustc_compile(&rust_code);
    assert!(
        rustc_ok,
        "Generated Rust should compile with rustc:\n{}",
        rustc_stderr
    );
}

#[test]
fn test_u32_compound_sub_assign() {
    let source = r#"
fn decrement() -> u32 {
    let mut x: u32 = 10
    x -= 1
    return x
}

fn main() {
    let _x = decrement()
}
"#;

    let (wj_ok, rust_code, _stderr) = compile_and_get_rust(source, "u32_sub");

    assert!(wj_ok, "Windjammer should compile");

    assert!(
        rust_code.contains("1_u32"),
        "x -= 1 where x: u32 should generate 1_u32, got:\n{}",
        rust_code
    );

    let (rustc_ok, rustc_stderr) = rustc_compile(&rust_code);
    assert!(
        rustc_ok,
        "Generated Rust should compile with rustc:\n{}",
        rustc_stderr
    );
}

// Note: test_u32_compound_add_assign_return_flow (count = 0u32, count += 1) would require
// either: (a) lexer preserving type suffix in 0u32, or (b) stronger return-flow analysis.
// For now, use explicit type: let mut count: u32 = 0

#[test]
fn test_vec_index_compound_assign() {
    let source = r#"
fn update_vec() -> Vec<u32> {
    let mut vec: Vec<u32> = vec![0, 1, 2]
    vec[0] += 1
    vec
}

fn main() {
    let _x = update_vec()
}
"#;

    let (wj_ok, rust_code, _stderr) = compile_and_get_rust(source, "vec_index");

    assert!(wj_ok, "Windjammer should compile");

    assert!(
        rust_code.contains("1_u32"),
        "vec[0] += 1 where vec: Vec<u32> should generate 1_u32, got:\n{}",
        rust_code
    );

    let (rustc_ok, rustc_stderr) = rustc_compile(&rust_code);
    assert!(
        rustc_ok,
        "Generated Rust should compile with rustc:\n{}",
        rustc_stderr
    );
}
