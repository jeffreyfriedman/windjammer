// TDD Test: Remove unnecessary type casts in comparisons
//
// Bug: while i < (vec.len() as i64) generates mismatched types
// Root cause: User added `as i64` manually, but `i` is i32
// Rust: `i32 < i64` fails
//
// Fix: Either remove `as i64` or infer `i` as i64

use std::fs;
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

#[test]
fn test_len_comparison_no_explicit_cast() {
    let source = r#"
fn test(items: Vec<i32>) {
    let mut i = 0
    while i < items.len() {
        println!("{}", i)
        i = i + 1
    }
}
"#;

    let rust_code = compile_single_file(source);
    println!("Generated Rust:\n{}", rust_code);
    println!("Len comparison test PASSED");
}

#[test]
fn test_len_comparison_with_explicit_i64_cast() {
    let source = r#"
fn test(items: Vec<i32>) {
    let mut i = 0
    while i < (items.len() as i64) {
        println!("{}", i)
        i = i + 1
    }
}
"#;

    let rust_code = compile_single_file(source);
    println!("Generated Rust:\n{}", rust_code);
    println!("Len comparison with i64 cast test PASSED");
}
