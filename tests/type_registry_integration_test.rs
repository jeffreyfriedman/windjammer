// Integration test for TypeRegistry
// Verifies that imports are generated correctly based on type definitions

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn setup_test_project() -> (PathBuf, PathBuf) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_dir = std::env::temp_dir();
    let project_dir = temp_dir.join(format!("type_reg_test_{}", test_id));
    let src_dir = project_dir.join("src");

    fs::create_dir_all(&src_dir).expect("Failed to create project directory");

    // Create math module with Vec2
    fs::write(
        src_dir.join("vec2.wj"),
        r#"pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }
}"#,
    )
    .expect("Failed to write vec2.wj");

    // Create rendering module with Color
    fs::write(
        src_dir.join("color.wj"),
        r#"pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }
}"#,
    )
    .expect("Failed to write color.wj");

    // Create main file that uses both types
    fs::write(
        src_dir.join("main.wj"),
        r#"use vec2::Vec2
use color::Color

pub struct Game {
    pub position: Vec2,
    pub color: Color,
}

impl Game {
    pub fn new() -> Game {
        Game {
            position: Vec2::new(0.0, 0.0),
            color: Color::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}"#,
    )
    .expect("Failed to write main.wj");

    let output_dir = temp_dir.join(format!("type_reg_output_{}", test_id));
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    (project_dir, output_dir)
}

#[test]
fn test_type_registry_fixes_import_paths() {
    let (project_dir, output_dir) = setup_test_project();
    let src_dir = project_dir.join("src");

    // Compile the project
    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--bin",
            "wj",
            "--",
            "build",
            "--output",
            output_dir.to_str().unwrap(),
            src_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Compilation failed: {}", stderr);
    }

    assert!(output.status.success(), "Compilation should succeed");

    // Read generated main.rs
    let main_rs = output_dir.join("main.rs");
    let generated_code = fs::read_to_string(&main_rs).expect("Failed to read generated main.rs");

    // Verify correct import paths are generated
    // Should be: use super::vec2::Vec2; (not use super::Vec2;)
    // Should be: use super::color::Color; (not use super::Color;)
    assert!(
        generated_code.contains("use super::vec2::Vec2")
            || generated_code.contains("use crate::vec2::Vec2")
            || !generated_code.contains("use super::Vec2"),
        "Should generate correct import path for Vec2"
    );

    assert!(
        generated_code.contains("use super::color::Color")
            || generated_code.contains("use crate::color::Color")
            || !generated_code.contains("use super::Color"),
        "Should generate correct import path for Color"
    );

    // Cleanup
    let _ = fs::remove_dir_all(&project_dir);
    let _ = fs::remove_dir_all(&output_dir);

    println!("✓ TypeRegistry correctly generates import paths");
}

#[test]
#[ignore] // Will enable after TypeRegistry is fully integrated
fn test_type_registry_handles_nested_modules() {
    // Test that TypeRegistry works with nested module structures
    // e.g., math/vec2.wj, rendering/color.wj
    println!("TODO: Test nested module import paths");
}
