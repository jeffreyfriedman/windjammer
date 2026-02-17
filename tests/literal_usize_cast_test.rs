/// TDD Test: Integer literals don't need `as usize` cast in index and usize arithmetic
///
/// Bug: The compiler generates `arr[0 as usize]` instead of `arr[0]`, and
/// `items.len() - 1 as usize` instead of `items.len() - 1`.
/// Rust infers integer literal types from context, so these casts are unnecessary
/// and trigger clippy warnings.
///
/// Root Cause: The index expression handler and binary expression handler always
/// add `as usize` for integer literals, without checking if the context already
/// provides the correct type inference.
///
/// Fix: Skip `as usize` when the expression is an integer literal (non-negative),
/// since Rust will infer it as `usize` from context.

use std::io::Write;
use std::process::Command;

fn compile_wj(source: &str) -> String {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let wj_path = dir.path().join("test.wj");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();

    let mut file = std::fs::File::create(&wj_path).unwrap();
    file.write_all(source.as_bytes()).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "--no-cargo",
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run wj");

    let rs_path = out_dir.join("test.rs");
    if rs_path.exists() {
        std::fs::read_to_string(&rs_path).unwrap()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "No output file generated.\nstdout: {}\nstderr: {}",
            stdout, stderr
        );
    }
}

#[test]
fn test_no_usize_cast_on_integer_literal_index() {
    // arr[0] should NOT generate arr[0 as usize]
    let source = r#"
pub fn first_element(arr: Vec<i32>) -> i32 {
    arr[0]
}

pub fn third_element(arr: Vec<i32>) -> i32 {
    arr[2]
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        !generated.contains("0 as usize"),
        "Integer literal 0 in index position should not have `as usize` cast.\nGenerated:\n{}",
        generated
    );
    assert!(
        !generated.contains("2 as usize"),
        "Integer literal 2 in index position should not have `as usize` cast.\nGenerated:\n{}",
        generated
    );
    // But it SHOULD still compile (literals infer to usize in index context)
    assert!(
        generated.contains("arr[0]") || generated.contains("[0]"),
        "Should index with bare literal.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_no_usize_cast_on_literal_in_usize_arithmetic() {
    // items.len() - 1 should NOT generate items.len() - 1 as usize
    let source = r#"
pub fn last_index(items: Vec<i32>) -> usize {
    items.len() - 1
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        !generated.contains("1 as usize"),
        "Integer literal in arithmetic with usize should not have `as usize` cast.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_variable_index_still_gets_usize_cast() {
    // arr[idx] where idx is i32 SHOULD get `as usize` since i32 != usize
    let source = r#"
pub fn element_at(arr: Vec<i32>, idx: i32) -> i32 {
    arr[idx]
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("idx as usize"),
        "Variable index should still get `as usize` cast when type is i32.\nGenerated:\n{}",
        generated
    );
}
