//! Verification tests for ownership tracker population.
//!
//! Ensures ALL variable bindings are registered with the ownership tracker:
//! - Let bindings (simple and destructuring)
//! - For loop variables
//! - Match arm bindings
//! - If-let bindings
//! - Function parameters (verified in function_generation)

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_to_rust(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.to_string());
    }

    let generated_path = out_dir.join("test.rs");
    fs::read_to_string(&generated_path).map_err(|e| e.to_string())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_binding_registered() {
    // If tracker not populated, will get E0614 or E0308 when using x in y = x + 1
    let src = r#"
pub fn process() -> i32 {
    let x = 5
    let y = x + 1
    y
}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("let x = 5"), "Expected let x = 5 in output");
    assert!(result.contains("let y = x + 1"), "Expected let y = x + 1 in output");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_from_param_registered() {
    // let x = param where param is borrowed - x should be usable
    let src = r#"
pub fn process(data: str) -> int {
    let x = data
    x.len() as int
}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("let x = data"), "Expected let binding");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop_variable_registered() {
    let src = r#"
pub fn sum_items() -> i32 {
    let items = [1, 2, 3]
    let mut total = 0
    for item in items {
        total = total + item
    }
    total
}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("for "), "Expected for loop");
    assert!(result.contains("item"), "Expected loop variable");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_arm_binding_registered() {
    let src = r#"
pub fn process(opt: Option<i32>) -> i32 {
    match opt {
        Some(x) => x + 1
        None => 0
    }
}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("Some(x)"), "Expected match arm with binding");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_let_binding_registered() {
    let src = r#"
pub fn process(opt: Option<i32>) -> i32 {
    if let Some(x) = opt {
        x + 1
    } else {
        0
    }
}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("if let Some(x)"), "Expected if let with binding");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_tuple_destructuring_registered() {
    let src = r#"
pub fn process() -> i32 {
    let t = (1, 2, 3)
    let (a, b, c) = t
    a + b + c
}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("let (a, b, c)"), "Expected tuple destructuring");
}
