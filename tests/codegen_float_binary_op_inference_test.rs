/// TDD: Float literal inference in binary operations
/// BUG: `f32_value * 1.414` generates `f32_value * 1.414_f64` (type mismatch)
/// FIX: Infer float literals from operand types in binary operations

use std::fs;
use std::process::Command;

#[test]
fn test_float_literal_in_f32_binary_op() {
    let wj_source = r#"
fn get_distance(x: f32, y: f32) -> f32 {
    let dx = x * x
    let dy = y * y
    let dist_sq = dx + dy
    dist_sq * 1.414  // Should be 1.414_f32, not 1.414_f64
}

fn main() {
    let d = get_distance(3.0, 4.0)
    println!("{}", d)
}
"#;

    let output_dir = "/tmp/wj_test_float_binop";
    fs::create_dir_all(output_dir).unwrap();
    fs::write(format!("{}/test.wj", output_dir), wj_source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj")).args([
            "build",
            "--target",
            "rust",
            "--no-cargo",
            &format!("{}/test.wj", output_dir),
            "--output",
            output_dir,
        ])
        .current_dir("/Users/jeffreyfriedman/src/wj/windjammer")
        .output()
        .expect("Failed to run wj");

    assert!(
        output.status.success(),
        "Compilation should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rust_code = fs::read_to_string(format!("{}/test.rs", output_dir))
        .expect("Generated Rust file not found");

    // The literal 1.414 in `dist_sq * 1.414` should be f32 (from dist_sq: f32)
    assert!(
        !rust_code.contains("1.414_f64"),
        "1.414 should NOT be f64 when multiplying f32, got:\n{}",
        rust_code
    );

    // Verify Rust compilation succeeds (no type mismatch errors)
    let cargo_toml = r#"
[package]
name = "test"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "test"
path = "test.rs"
"#;
    fs::write(format!("{}/Cargo.toml", output_dir), cargo_toml).unwrap();

    let rust_build = Command::new("cargo")
        .args(&["build", "--release"])
        .current_dir(output_dir)
        .output()
        .expect("Failed to build Rust");

    assert!(
        rust_build.status.success(),
        "Rust compilation should succeed (no f32/f64 mixing), stderr: {}",
        String::from_utf8_lossy(&rust_build.stderr)
    );
}
