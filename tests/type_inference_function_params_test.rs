// TDD Test: Float literal inference from function parameter types
//
// Bug: Vec3::new(x, 0.0, z) generates 0.0_f64 instead of 0.0_f32
// Expected: Look up Vec3::new signature → (f32, f32, f32) → constrain args
//
// Dogfooding Win: Constructors are everywhere in game code

use std::fs;
use std::process::Command;

#[test]
fn test_function_param_float_inference() {
    let wj_source = r#"
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

fn create_vector(x: f32, z: f32) -> Vec3 {
    Vec3::new(x, 0.0, z)
}
"#;

    let output_dir = "/tmp/wj_test_func_params";
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

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Compilation should succeed, stderr: {}",
        stderr
    );

    let rust_code = fs::read_to_string(format!("{}/test.rs", output_dir))
        .expect("Generated Rust file not found");

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 0.0 should be f32 (from Vec3::new(f32, f32, f32))
    assert!(
        !rust_code.contains("Vec3::new(x, 0.0_f64") && !rust_code.contains("new(x, 0_f64"),
        "0.0 should NOT be f64 when passed to Vec3::new(f32, f32, f32), got:\n{}",
        rust_code
    );
}
