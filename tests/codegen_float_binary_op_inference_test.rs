/// TDD: Float literal inference in binary operations
/// BUG: `f32_value * 1.414` generates `f32_value * 1.414_f64` (type mismatch)
/// FIX: Infer float literals from operand types in binary operations
use std::fs;
use std::process::Command;
use tempfile::TempDir;

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

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, wj_source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            "--target",
            "rust",
            "--no-cargo",
            test_file.to_str().unwrap(),
            "--output",
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run wj");

    assert!(
        output.status.success(),
        "Compilation should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rs_file = temp_dir.path().join("test.rs");
    let rust_code = fs::read_to_string(&rs_file).expect("Generated Rust file not found");

    // The literal 1.414 in `dist_sq * 1.414` should be f32 (from dist_sq: f32)
    assert!(
        !rust_code.contains("1.414_f64"),
        "1.414 should NOT be f64 when multiplying f32, got:\n{}",
        rust_code
    );

    let cargo_toml = r#"
[package]
name = "test"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "test"
path = "test.rs"

[workspace]
"#;
    let cargo_toml_path = temp_dir.path().join("Cargo.toml");
    fs::write(&cargo_toml_path, cargo_toml).unwrap();

    let rust_build = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to build Rust");

    assert!(
        rust_build.status.success(),
        "Rust compilation should succeed (no f32/f64 mixing), stderr: {}",
        String::from_utf8_lossy(&rust_build.stderr)
    );
}
