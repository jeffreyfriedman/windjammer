//! TDD Test: Cross-crate type inference for external struct fields
//!
//! Problem: When compiling game.wj that uses windjammer-game-core (external crate),
//! struct field types like Vec3.x: f32 aren't available, breaking float inference.
//!
//! Solution: Load external crate metadata (metadata.json) when compiling with dependencies.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_library_emits_metadata_json() {
    // Step 1: Verify that building a library emits metadata.json
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let lib_dir = temp_path.join("mylib");
    fs::create_dir_all(lib_dir.join("src")).unwrap();
    fs::write(
        lib_dir.join("src").join("vec3.wj"),
        r#"
pub struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }
}
"#,
    )
    .unwrap();

    let lib_output = lib_dir.join("build");
    fs::create_dir_all(&lib_output).unwrap();

    let wj_status = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(&[
            "build",
            lib_dir.join("src").to_str().unwrap(),
            "-o",
            lib_output.to_str().unwrap(),
            "--library",
            "--no-cargo",
        ])
        .current_dir(temp_path)
        .status()
        .expect("Failed to run wj build for lib");

    assert!(wj_status.success(), "Lib build should succeed");

    let metadata_path = lib_output.join("metadata.json");
    assert!(
        metadata_path.exists(),
        "metadata.json should be generated when building library. Path: {}",
        metadata_path.display()
    );

    let metadata_content = fs::read_to_string(&metadata_path).unwrap();
    assert!(
        metadata_content.contains("Vec3"),
        "metadata.json should contain Vec3 struct"
    );
    assert!(
        metadata_content.contains("x") && metadata_content.contains("f32"),
        "metadata.json should contain Vec3 field types"
    );
}

#[test]
fn test_external_struct_field_inference_with_metadata_flag() {
    // Step 2: Verify that --metadata flag loads external crate metadata for type inference
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create lib
    let lib_dir = temp_path.join("mylib");
    fs::create_dir_all(lib_dir.join("src")).unwrap();
    fs::write(
        lib_dir.join("src").join("vec3.wj"),
        r#"
pub struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}
"#,
    )
    .unwrap();

    let lib_output = lib_dir.join("build");
    fs::create_dir_all(&lib_output).unwrap();

    Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(&[
            "build",
            lib_dir.join("src").to_str().unwrap(),
            "-o",
            lib_output.to_str().unwrap(),
            "--library",
            "--no-cargo",
        ])
        .current_dir(temp_path)
        .status()
        .expect("Lib build failed");

    // Create app that uses external crate
    let app_dir = temp_path.join("myapp");
    fs::create_dir_all(app_dir.join("src")).unwrap();
    fs::write(
        app_dir.join("src").join("main.wj"),
        r#"
use mylib::vec3::Vec3

fn update(pos: Vec3) {
    let offset = 10.0
    let new_x = pos.x + offset
}
"#,
    )
    .unwrap();

    let app_output = app_dir.join("build");
    fs::create_dir_all(&app_output).unwrap();

    // Build app with --metadata to load external crate struct fields
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(&[
            "build",
            app_dir.join("src").join("main.wj").to_str().unwrap(),
            "-o",
            app_output.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("mylib={}", lib_output.display()),
        ])
        .current_dir(temp_path)
        .output()
        .expect("Failed to run wj build for app");

    if !output.status.success() {
        panic!(
            "App build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let main_rs = app_output.join("main.rs");
    assert!(main_rs.exists(), "main.rs should be generated");
    let rust_code = fs::read_to_string(&main_rs).unwrap();
    assert!(
        rust_code.contains("10.0_f32") || rust_code.contains("10.0f32"),
        "Should infer offset as f32 from Vec3.x type. Generated:\n{}",
        rust_code
    );
}
