//! TDD: Backward type propagation for float literals in variable initialization
//!
//! Problem: Variables initialized with float literals get wrong type when used later.
//! Example: `let offset_x = 0.0` defaults to f64, but `self.player.position.x + offset_x`
//! needs f32 + f32.
//!
//! Root Cause: Inference runs in single pass, doesn't propagate constraints backward
//! from usage site to initialization.

use tempfile::TempDir;
use windjammer::{build_project, CompilationTarget};

fn compile_to_rust(source: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    let output_dir = test_dir.join("build");

    std::fs::create_dir_all(&output_dir).expect("Failed to create output dir");
    std::fs::write(&input_file, source).expect("Failed to write source file");

    build_project(&input_file, &output_dir, CompilationTarget::Rust, true)
        .map_err(|e| format!("Windjammer compilation failed: {}", e))?;

    let output_file = output_dir.join("test.rs");
    let rust_code = std::fs::read_to_string(&output_file)
        .map_err(|e| format!("Failed to read generated file: {}", e))?;

    Ok(rust_code)
}

fn verify_rust_compiles(rust_code: &str) -> Result<(), String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let rust_file = test_dir.join("test.rs");
    std::fs::write(&rust_file, rust_code).expect("Failed to write Rust file");

    let check = std::process::Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            rust_file.to_str().unwrap(),
            "-o",
            test_dir.join("test.rlib").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    if check.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&check.stderr).to_string())
    }
}

#[test]
fn test_variable_used_with_f32_field() {
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Player {
    pub position: Vec3,
}

pub struct Camera {
    pub player: Player,
}

impl Camera {
    pub fn update_camera(self) {
        let offset_x = 0.0
        let offset_y = 5.0
        let offset_z = -10.0

        let cam_x = self.player.position.x + offset_x
        let cam_y = self.player.position.y + offset_y
        let cam_z = self.player.position.z + offset_z
    }
}
"#;

    let rust = compile_to_rust(source).unwrap();

    // offset variables should be inferred as f32 (backward propagation from usage)
    assert!(
        rust.contains("0.0_f32") || rust.contains("0.0f32"),
        "offset_x = 0.0 should generate 0.0_f32 when used with f32 field, got:\n{}",
        rust
    );
    assert!(
        rust.contains("5.0_f32") || rust.contains("5.0f32"),
        "offset_y = 5.0 should generate 5.0_f32, got:\n{}",
        rust
    );
    assert!(
        rust.contains("-10.0_f32") || rust.contains("-10.0f32"),
        "offset_z = -10.0 should generate -10.0_f32, got:\n{}",
        rust
    );

    // Verify it compiles
    verify_rust_compiles(&rust).expect("Generated Rust should compile");
}
