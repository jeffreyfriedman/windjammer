/// Test that if-else expressions don't get semicolons added to branches
///
/// Bug: When if-else is used as an expression (assigned to variable or returned),
/// transpiler was adding semicolons to branches, turning them into statements.
/// This causes type errors: expected Option<T>, found ()
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_else_expression_no_semicolons() {
    let source = r#"
pub fn get_last_selected(is_empty: bool, value: i64) -> Option<i64> {
    let result = if is_empty {
        None
    } else {
        Some(value)
    };
    result
}

pub fn get_last_inline(is_empty: bool, value: i64) -> Option<i64> {
    if is_empty {
        None
    } else {
        Some(value)
    }
}
"#;

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
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        panic!("Compilation failed:\n{}", stderr);
    }

    // Read generated Rust
    let generated_rs = out_dir.join("test.rs");
    let result = fs::read_to_string(&generated_rs).expect("Failed to read generated Rust");

    // Should NOT add semicolons to if-else branches when used as expression
    // The generated code should compile without type errors
    assert!(
        !result.contains("None;\n"),
        "Should not add semicolon after None in if-else expression"
    );

    // Verify the code actually compiles (rustc should succeed)
    let rustc_output = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg("--edition=2021")
        .arg(&generated_rs)
        .arg("-o")
        .arg(temp_dir.path().join("test.rlib"))
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        panic!(
            "Generated Rust has type errors:\n{}",
            String::from_utf8_lossy(&rustc_output.stderr)
        );
    }
}
