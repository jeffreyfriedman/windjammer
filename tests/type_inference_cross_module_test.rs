// TDD Test: Float literal inference across module boundaries
//
// Bug: Vec3::new(x, 0.0, z) generates 0.0_f64 when Vec3 is imported from another file
// Expected: Load Vec3::new signature from metadata → constrain args to f32
//
// Dogfooding Win: 99% of game code imports types from other modules

use std::fs;
use std::process::Command;

#[test]
fn test_cross_module_function_signature() {
    let output_dir = "/tmp/wj_test_cross_module";
    fs::create_dir_all(&format!("{}/math", output_dir)).unwrap();

    // Module 1: Define Vec3
    let vec3_source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }
}
"#;
    fs::write(format!("{}/math/vec3.wj", output_dir), vec3_source).unwrap();

    // Module 2: Use Vec3
    let usage_source = r#"
use crate::math::vec3::Vec3

fn create_vector(x: f32, z: f32) -> Vec3 {
    Vec3::new(x, 0.0, z)
}
"#;
    fs::write(format!("{}/usage.wj", output_dir), usage_source).unwrap();

    // Compile Vec3 first
    let output1 = Command::new("cargo")
        .args(&[
            "run",
            "--release",
            "--",
            "build",
            "--target",
            "rust",
            "--no-cargo",
            &format!("{}/math/vec3.wj", output_dir),
            "--output",
            &format!("{}/math", output_dir),
        ])
        .current_dir("/Users/jeffreyfriedman/src/wj/windjammer")
        .output()
        .expect("Failed to compile vec3.wj");

    assert!(output1.status.success(), "Vec3 compilation failed");

    // Compile usage.wj (should load Vec3 metadata)
    let output2 = Command::new("cargo")
        .args(&[
            "run",
            "--release",
            "--",
            "build",
            "--target",
            "rust",
            "--no-cargo",
            &format!("{}/usage.wj", output_dir),
            "--output",
            output_dir,
        ])
        .current_dir("/Users/jeffreyfriedman/src/wj/windjammer")
        .output()
        .expect("Failed to compile usage.wj");

    let stderr = String::from_utf8_lossy(&output2.stderr);
    
    eprintln!("=== STDERR FROM WJ BUILD ===\n{}\n===", stderr);
    
    assert!(
        output2.status.success(),
        "Usage compilation should succeed, stderr: {}",
        stderr
    );

    let rust_code = fs::read_to_string(format!("{}/usage.rs", output_dir))
        .expect("Generated Rust file not found");

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 0.0 should be f32 (from Vec3::new signature in other file)
    assert!(
        !rust_code.contains("0.0_f64") && !rust_code.contains("0_f64"),
        "0.0 should NOT be f64 when passed to Vec3::new (even from other file), got:\n{}",
        rust_code
    );
}
