#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

// Integration test for TypeRegistry
// Verifies that imports are generated correctly based on type definitions

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_type_registry_fixes_import_paths() {
    let tmp = TempDir::new().unwrap();
    let src_dir = tmp.path().join("src");
    let output_dir = tmp.path().join("output");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

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
    .unwrap();

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
    .unwrap();

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
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
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

    let generated_code =
        fs::read_to_string(output_dir.join("main.rs")).expect("Failed to read generated main.rs");

    assert!(
        generated_code.contains("use super::vec2::Vec2")
            || generated_code.contains("use super::Vec2")
            || generated_code.contains("use crate::Vec2"),
        "Should generate import path for Vec2.\nGenerated code:\n{}",
        generated_code
    );

    assert!(
        generated_code.contains("use super::color::Color")
            || generated_code.contains("use super::Color")
            || generated_code.contains("use crate::Color"),
        "Should generate import path for Color.\nGenerated code:\n{}",
        generated_code
    );
}

#[test]
fn test_type_registry_handles_nested_modules() {
    println!("TODO: Test nested module import paths");
}
