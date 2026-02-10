// TDD TEST: Match arms should only have semicolons in statement context, not expression context
//
// BUG: Match arms in expression context (returning a value) are getting semicolons,
//      preventing them from returning values
//
// FIX: Only add semicolons in match arms when the function returns void

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_arm_in_expression_context() {
    let code = r#"
pub fn get_value(x: i32) -> i32 {
    match x {
        1 => {
            42
        },
        _ => {
            0
        },
    }
}

fn main() {
    let v = get_value(1);
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path();
    let wj_file = output_dir.join("test.wj");
    fs::write(&wj_file, code).unwrap();

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
    println!("Generated Rust:\n{}", rust_code);

    // Should NOT have semicolons after the values (expression context)
    assert!(
        rust_code.contains("42\n")
            || rust_code.contains("42 }")
            || rust_code.contains("        },"),
        "Match arm in expression context should not have semicolon after 42"
    );
    assert!(
        !rust_code.contains("42;\n"),
        "Match arm in expression context should NOT have semicolon:\n{}",
        rust_code
    );

    println!("✅ Match arms in expression context have no semicolons");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_arm_in_statement_context() {
    let code = r#"
pub fn process(x: i32) {
    match x {
        1 => {
            do_something(42);
        },
        _ => {
            do_something(0);
        },
    }
}

pub fn do_something(x: i32) -> i32 {
    x * 2
}

fn main() {
    process(1);
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path();
    let wj_file = output_dir.join("test.wj");
    fs::write(&wj_file, code).unwrap();

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
    println!("Generated Rust:\n{}", rust_code);

    // SHOULD have semicolons (statement context, void return)
    assert!(
        rust_code.contains("do_something(42);"),
        "Match arm in statement context should have semicolon"
    );

    println!("✅ Match arms in statement context have semicolons");
}
