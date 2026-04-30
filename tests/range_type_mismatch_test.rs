// TDD Test: Range type unification in for loops
//
// Bug: for i in 0..vec.len() generates 0_i32..vec.len() (usize)
// This creates a Range<T> where T is ambiguous (i32 vs usize)
//
// Fix: When range bounds have different integer types, unify them to a common type
//      Prefer usize for ranges ending with .len()

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
fn test_range_with_vec_len() {
    let test_wj = r#"
fn test(items: Vec<i32>) {
    for i in 0..items.len() {
        println!("{}", i)
    }
}
"#;

    let rust_code = compile_single_file(test_wj);

    println!("Generated Rust:\n{}", rust_code);

    // Verify range has unified types (both usize)
    assert!(
        rust_code.contains("0_usize..items.len()") || rust_code.contains("0..items.len()"), // Rust infers 0 as usize from context
        "Should unify range types: 0_usize..items.len() or 0..items.len()\nGenerated:\n{}",
        rust_code
    );

    // Verify does NOT generate mismatched types
    assert!(
        !rust_code.contains("0_i32..items.len()"),
        "Should NOT generate: 0_i32..items.len() (type mismatch)\nGenerated:\n{}",
        rust_code
    );

    println!("✅ Range with vec.len() test PASSED - correct type unification!");
}

#[test]
fn test_range_with_field_len() {
    let test_wj = r#"
struct Container {
    items: Vec<String>
}

impl Container {
    fn process(self) {
        for i in 0..self.items.len() {
            println!("{}", i)
        }
    }
}
"#;

    let rust_code = compile_single_file(test_wj);

    println!("Generated Rust:\n{}", rust_code);

    // Should unify to usize (matching .len() return type)
    assert!(
        rust_code.contains("0..self.items.len()")
            || rust_code.contains("0_usize..self.items.len()"),
        "Should unify range types for field.len()\nGenerated:\n{}",
        rust_code
    );

    // Should NOT have type mismatch
    assert!(
        !rust_code.contains("0_i32..self.items.len()"),
        "Should NOT generate: 0_i32..self.items.len()\nGenerated:\n{}",
        rust_code
    );

    println!("✅ Range with field.len() test PASSED - correct type unification!");
}
