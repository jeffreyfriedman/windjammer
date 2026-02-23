// BUG: Parser reports "Unexpected Break token" in match arm
//
// DISCOVERED DURING: Dogfooding astar_grid.wj
//
// PROBLEM:
// Windjammer source: `match x { Some(v) => {...}, None => break }`
// Parser fails: "Unexpected token in expression: Break (at token position N)"
//
// ROOT CAUSE:
// Match arm body without braces is parsed as expression. Expression parser
// does not handle break/continue/return (control-flow statements).
//
// FIX:
// When match arm body starts with Break, Continue, or Return, parse as
// statement and wrap in block (same as assignment handling).

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

fn compile_to_rust(source: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, source).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(get_wj_compiler())
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
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let generated_path = out_dir.join("test.rs");
    fs::read_to_string(&generated_path).map_err(|e| format!("Failed to read: {}", e))
}

#[test]
fn test_break_in_match_arm() {
    // Minimal: loop with match, None => break (from astar_grid.wj path reconstruction)
    let source = r#"
fn test() {
    let mut x = 0
    loop {
        match x {
            0 => { x = 1 },
            1 => break,
        }
    }
}
"#;

    let result = compile_to_rust(source);
    assert!(
        result.is_ok(),
        "break in match arm should compile. Error: {:?}",
        result.err()
    );

    let rust_code = result.unwrap();
    assert!(
        rust_code.contains("break"),
        "Generated Rust should contain break"
    );
}

#[test]
fn test_continue_in_match_arm() {
    let source = r#"
fn test() {
    let mut i = 0
    while i < 10 {
        match i % 2 {
            0 => continue,
            _ => { i = i + 1 },
        }
        i = i + 1
    }
}
"#;

    let result = compile_to_rust(source);
    assert!(
        result.is_ok(),
        "continue in match arm should compile. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_return_in_match_arm() {
    let source = r#"
fn test(x: i32) -> i32 {
    match x {
        0 => return 0,
        _ => x + 1,
    }
}
"#;

    let result = compile_to_rust(source);
    assert!(
        result.is_ok(),
        "return in match arm should compile. Error: {:?}",
        result.err()
    );
}
