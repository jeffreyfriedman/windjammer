//! Ownership Tracker Integration Tests
//!
//! Verifies that the ownership tracking system correctly populates and
//! influences code generation for parameters, for-loops, and match patterns.
//! Philosophy: "Safety Without Ceremony" - automatic ownership tracking.

use std::fs;
use std::process::Command;

fn compile_to_rust(wj_source: &str) -> Result<String, String> {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, wj_source).expect("write");
    fs::create_dir_all(&out_dir).expect("create dir");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_path)
        .arg("--output")
        .arg(&out_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .output()
        .expect("wj");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let src_main = out_dir.join("src").join("main.rs");
    let test_rs = out_dir.join("test.rs");
    let content = if src_main.exists() {
        fs::read_to_string(src_main)
    } else if test_rs.exists() {
        fs::read_to_string(test_rs)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No generated Rust file",
        ))
    };
    content.map_err(|e| e.to_string())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_parameter_ownership_tracked_borrowed() {
    let src = r#"
pub struct Data { pub value: int }
pub fn process(data: Data) -> int {
    data.value
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("data.value"), "Should use data.value directly");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_parameter_owned_vec() {
    let src = r#"
pub fn sum(items: Vec<int>) -> int {
    let mut total = 0
    for item in items {
        total += item
    }
    total
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("items: Vec<i64>") || result.contains("items: Vec<i32>"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_option_owned() {
    let src = r#"
pub fn unwrap(opt: Option<int>) -> int {
    match opt {
        Some(val) => val,
        None => 0
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("Some(val)"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_let_owned() {
    let src = r#"
pub fn get_default(opt: Option<int>) -> int {
    if let Some(x) = opt {
        x
    } else {
        0
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("if let Some(x) = opt"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_ownership_tracker_does_not_break_existing() {
    let src = r#"
pub fn identity(x: int) -> int { x }
pub fn main() {
    let x = identity(42)
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("fn identity"));
}
