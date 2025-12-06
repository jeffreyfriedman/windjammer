//! Test: super:: import path flattening
//!
//! Dogfooding Win: When Windjammer source files use nested super:: paths like
//! `use super::super::math::vec3::Vec3`, the compiler should flatten these
//! to `use super::vec3::Vec3` since all generated Rust files are siblings
//! in the same directory.

use std::fs;
use std::process::Command;

#[test]
fn test_super_super_import_flattens_to_super() {
    // Create the test source file
    let source = r#"
// Simulating: src_wj/rendering/camera3d.wj importing from src_wj/math/vec3.wj
use super::super::math::vec3::Vec3

@derive(Copy, Clone, Debug)
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Camera3D {
    pub position: Vec3,
}
"#;

    // Write the test file
    let input_dir = "tests/generated/super_import_test";
    fs::create_dir_all(input_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test_super_import.wj", input_dir);
    fs::write(&input_file, source).expect("Failed to write source file");

    // Run the compiler
    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            &input_file,
            "--output",
            input_dir,
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        panic!(
            "Compiler failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Read the generated Rust code
    let generated = fs::read_to_string(format!("{}/test_super_import.rs", input_dir))
        .expect("Failed to read output");

    println!("Generated code:\n{}", generated);

    // The import should be flattened from super::super::math::vec3::Vec3 to super::vec3::Vec3
    assert!(
        generated.contains("use super::vec3::Vec3;"),
        "Expected flattened import 'use super::vec3::Vec3;' but got:\n{}",
        generated
    );

    // Should NOT contain the original nested path
    assert!(
        !generated.contains("super::super"),
        "Should not contain nested super:: but got:\n{}",
        generated
    );
}

#[test]
fn test_single_super_import_preserved() {
    // A simple super:: import should still work
    let source = r#"
use super::vec3::Vec3

@derive(Copy, Clone, Debug)
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Camera3D {
    pub position: Vec3,
}
"#;

    let input_dir = "tests/generated/single_super_test";
    fs::create_dir_all(input_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test_single_super.wj", input_dir);
    fs::write(&input_file, source).expect("Failed to write source file");

    // Run the compiler
    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            &input_file,
            "--output",
            input_dir,
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        panic!(
            "Compiler failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Read the generated Rust code
    let generated = fs::read_to_string(format!("{}/test_single_super.rs", input_dir))
        .expect("Failed to read output");

    println!("Generated code:\n{}", generated);

    // The import should remain as super::vec3::Vec3
    assert!(
        generated.contains("use super::vec3::Vec3;"),
        "Expected 'use super::vec3::Vec3;' but got:\n{}",
        generated
    );
}

#[test]
fn test_deeply_nested_super_import_flattens() {
    // Even deeper nesting should flatten correctly
    let source = r#"
// Simulating deeply nested: use super::super::super::core::types::MyType
use super::super::super::core::types::MyType

@derive(Copy, Clone, Debug)
pub struct MyType {
    pub value: i32,
}

pub struct Foo {
    pub value: MyType,
}
"#;

    let input_dir = "tests/generated/deep_super_test";
    fs::create_dir_all(input_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test_deep_super.wj", input_dir);
    fs::write(&input_file, source).expect("Failed to write source file");

    // Run the compiler
    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            &input_file,
            "--output",
            input_dir,
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        panic!(
            "Compiler failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Read the generated Rust code
    let generated = fs::read_to_string(format!("{}/test_deep_super.rs", input_dir))
        .expect("Failed to read output");

    println!("Generated code:\n{}", generated);

    // Should flatten to just super::types::MyType (last two segments)
    assert!(
        generated.contains("use super::types::MyType;"),
        "Expected flattened import 'use super::types::MyType;' but got:\n{}",
        generated
    );
}
