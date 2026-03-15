//! TDD Test: Statement expressions should NOT be borrowed
//!
//! Bug: When vec.push(item) is in an if block or match arm, the compiler incorrectly
//! generates &mut vec.push(item), producing &mut () instead of ().
//!
//! Root Cause: Method call object gets &mut prefix, but format! produces
//! "&mut obj.push(args)" which parses as &mut (obj.push(args)) due to operator precedence.
//!
//! Fix: Wrap object in parentheses when it starts with & or &mut for instance method calls:
//! (&mut obj).push(args) instead of &mut obj.push(args)

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_wj_to_rust(source: &str) -> (String, bool) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, source).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return (stderr, false);
    }

    let generated_path = out_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_path)
        .unwrap_or_else(|_| "Failed to read generated file".to_string());

    let rustc_output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            temp_dir.path().join("test.rlib").to_str().unwrap(),
        ])
        .arg(&generated_path)
        .output();

    let compiles = rustc_output
        .as_ref()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !compiles {
        if let Ok(ref out) = rustc_output {
            eprintln!("rustc stderr:\n{}", String::from_utf8_lossy(&out.stderr));
        }
    }

    (generated, compiles)
}

#[test]
fn test_vec_push_in_if_block_no_borrow() {
    // vec.push(item) in if block - should NOT generate &mut vec.push(item)
    let source = r#"
pub fn collect_items() -> Vec<i32> {
    let mut items: Vec<i32> = Vec::new()
    if true {
        items.push(1)
    }
    items
}
"#;
    let (result, compiles) = compile_wj_to_rust(source);
    assert!(
        compiles,
        "Should compile. Generated:\n{}\n\nrustc error above",
        result
    );
    // Must NOT have &mut items.push - that produces &mut ()
    assert!(
        !result.contains("&mut items.push("),
        "Should NOT add &mut to statement expression. Got:\n{}",
        result
    );
    // Should have (&mut items).push(1) or items.push(1) - both valid
    assert!(
        result.contains("items.push(1)") || result.contains("(&mut items).push(1)"),
        "Should have valid push call. Got:\n{}",
        result
    );
}

#[test]
fn test_vec_push_in_match_arm_no_borrow() {
    // vec.push(item) in match arm - statement position
    let source = r#"
pub fn process(x: i32) -> Vec<i32> {
    let mut result: Vec<i32> = Vec::new()
    match x {
        0 => result.push(0),
        1 => result.push(1),
        _ => result.push(-1),
    }
    result
}
"#;
    let (result, compiles) = compile_wj_to_rust(source);
    assert!(
        compiles,
        "Should compile. Generated:\n{}\n\nrustc error above",
        result
    );
    assert!(
        !result.contains("&mut result.push("),
        "Should NOT add &mut to statement expression in match arm. Got:\n{}",
        result
    );
}

#[test]
fn test_vec_push_other_unit_methods_in_statement_position() {
    // Other ()-returning methods: clear(), etc.
    let source = r#"
pub fn clear_vec() {
    let mut v: Vec<i32> = Vec::new()
    v.push(1)
    v.clear()
}
"#;
    let (result, compiles) = compile_wj_to_rust(source);
    assert!(
        compiles,
        "Should compile. Generated:\n{}\n\nrustc error above",
        result
    );
    assert!(
        !result.contains("&mut v.push("),
        "Should NOT add &mut to v.push(). Got:\n{}",
        result
    );
    assert!(
        !result.contains("&mut v.clear("),
        "Should NOT add &mut to v.clear(). Got:\n{}",
        result
    );
}
