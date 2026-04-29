//! TDD Test: Compound assignment type inference
//!
//! Bug: `count += 1` where count is u32 generates 1_i32 instead of 1_u32
//! Root Cause: IntInference doesn't constrain RHS literal to match LHS type for compound assignments
//!
//! Tests:
//! - u32 += literal -> literal must be u32
//! - i64 += literal -> literal must be i64
//! - f32 += literal -> (float inference, not int - skip)
//! - vec[i] += literal where Vec<u32> -> literal must be u32

use std::fs;
use std::process::Command;
use tempfile::tempdir;
use windjammer::{build_project_ext, CompilationTarget};

fn compile_single_file(source: &str) -> String {
    let src = tempdir().expect("tempdir for src");
    let out = tempdir().expect("tempdir for out");
    fs::write(src.path().join("test.wj"), source).expect("write test.wj");
    build_project_ext(
        src.path(),
        out.path(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("build_project_ext");
    let raw = fs::read_to_string(out.path().join("test.rs")).unwrap_or_default();
    raw.lines()
        .filter(|l| !l.contains("use super::"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn rustc_compile(rust_code: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir for rustc");
    fs::write(dir.path().join("test.rs"), rust_code).unwrap();

    let output = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(dir.path().join("test.rs"))
        .arg("--out-dir")
        .arg(dir.path())
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stderr)
}

#[test]
fn test_u32_compound_add_assign() {
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

    let rust_code = compile_single_file(source);

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

    let rust_code = compile_single_file(source);

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

    let rust_code = compile_single_file(source);

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

    let rust_code = compile_single_file(source);

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
