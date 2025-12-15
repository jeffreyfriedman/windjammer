//! Comprehensive test suite for import generation fixes
//!
//! Tests for the flat directory structure import generation bug fixes:
//! 1. math::Vec3 -> super::Vec3 (sibling modules)
//! 2. super::super::math::vec3::Vec3 -> super::Vec3 (path flattening)
//! 3. std::ops::* -> std::ops::* (not windjammer_runtime)

use std::fs;
use std::process::Command;

#[test]
fn test_math_module_import_uses_super() {
    // Test: use math::Vec3 should generate use super::Vec3;
    let source = r#"
use math::Vec3

pub struct Camera {
    pub position: Vec3,
}
"#;

    let input_dir = "tests/generated/math_import_test";
    fs::create_dir_all(input_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test_math.wj", input_dir);
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
    let generated =
        fs::read_to_string(format!("{}/test_math.rs", input_dir)).expect("Failed to read output");

    println!("Generated code:\n{}", generated);

    // CRITICAL: Should use super::Vec3, not math::Vec3
    assert!(
        generated.contains("use super::Vec3;"),
        "Expected 'use super::Vec3;' for flat directory structure but got:\n{}",
        generated
    );

    // Should NOT contain math::Vec3
    assert!(
        !generated.contains("use math::Vec3;"),
        "Should not contain 'use math::Vec3;' in flat directory but got:\n{}",
        generated
    );

    // Cleanup
    fs::remove_dir_all(input_dir).ok();
}

#[test]
fn test_collision2d_module_import_uses_super() {
    // Test: use collision2d::check_collision should generate use super::check_collision;
    let source = r#"
use collision2d::check_collision

pub fn test_collision() {
    // Function body
}
"#;

    let input_dir = "tests/generated/collision2d_import_test";
    fs::create_dir_all(input_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test_collision.wj", input_dir);
    fs::write(&input_file, source).expect("Failed to write source file");

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

    let generated = fs::read_to_string(format!("{}/test_collision.rs", input_dir))
        .expect("Failed to read output");

    println!("Generated code:\n{}", generated);

    // Should preserve module path to avoid ambiguity with glob re-exports
    assert!(
        generated.contains("use super::collision2d::check_collision;"),
        "Expected 'use super::collision2d::check_collision;' but got:\n{}",
        generated
    );

    fs::remove_dir_all(input_dir).ok();
}

#[test]
fn test_super_super_math_flattens_to_super() {
    // Test: use super::super::math::vec3::Vec3 should generate use super::Vec3;
    let source = r#"
use super::super::math::vec3::Vec3

pub struct Camera {
    pub position: Vec3,
}
"#;

    let input_dir = "tests/generated/super_super_flatten_test";
    fs::create_dir_all(input_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test_flatten.wj", input_dir);
    fs::write(&input_file, source).expect("Failed to write source file");

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

    let generated = fs::read_to_string(format!("{}/test_flatten.rs", input_dir))
        .expect("Failed to read output");

    println!("Generated code:\n{}", generated);

    // Should flatten to super::Vec3
    assert!(
        generated.contains("use super::Vec3;"),
        "Expected flattened 'use super::Vec3;' but got:\n{}",
        generated
    );

    // Should NOT contain super::super
    assert!(
        !generated.contains("super::super"),
        "Should not contain nested super:: but got:\n{}",
        generated
    );

    fs::remove_dir_all(input_dir).ok();
}

#[test]
fn test_std_ops_imports_use_rust_stdlib() {
    // Test: std::ops imports should NOT map to windjammer_runtime
    let source = r#"
use std::ops::Add
use std::ops::Sub
use std::ops::Mul
use std::ops::Div

pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Add for Vec2 {
    type Output = Vec2
    
    fn add(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}
"#;

    let input_dir = "tests/generated/std_ops_test";
    fs::create_dir_all(input_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test_ops.wj", input_dir);
    fs::write(&input_file, source).expect("Failed to write source file");

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

    let generated =
        fs::read_to_string(format!("{}/test_ops.rs", input_dir)).expect("Failed to read output");

    println!("Generated code:\n{}", generated);

    // CRITICAL: Must use std::ops, not windjammer_runtime::ops
    assert!(
        generated.contains("use std::ops::Add;"),
        "Expected 'use std::ops::Add;' but got:\n{}",
        generated
    );

    assert!(
        generated.contains("use std::ops::Sub;"),
        "Expected 'use std::ops::Sub;' but got:\n{}",
        generated
    );

    // Must NOT contain windjammer_runtime::ops
    assert!(
        !generated.contains("windjammer_runtime::ops"),
        "Should NOT use windjammer_runtime for std::ops but got:\n{}",
        generated
    );

    fs::remove_dir_all(input_dir).ok();
}

#[test]
fn test_entity_component_imports_use_super() {
    // Test: ECS module imports should use super::
    let source = r#"
use entity::Entity
use components::Transform

pub struct World {
    pub entities: Vec<Entity>,
}
"#;

    let input_dir = "tests/generated/ecs_import_test";
    fs::create_dir_all(input_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test_ecs.wj", input_dir);
    fs::write(&input_file, source).expect("Failed to write source file");

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

    let generated =
        fs::read_to_string(format!("{}/test_ecs.rs", input_dir)).expect("Failed to read output");

    println!("Generated code:\n{}", generated);

    // Should preserve module path for actual module files to avoid ambiguity
    assert!(
        generated.contains("use super::entity::Entity;"),
        "Expected 'use super::entity::Entity;' but got:\n{}",
        generated
    );

    assert!(
        generated.contains("use super::components::Transform;"),
        "Expected 'use super::components::Transform;' but got:\n{}",
        generated
    );

    fs::remove_dir_all(input_dir).ok();
}

#[test]
fn test_non_sibling_imports_unchanged() {
    // Test: Non-sibling module imports should NOT use super::
    let source = r#"
use std::collections::HashMap
use some_external_crate::Thing

pub struct MyStruct {
    pub data: HashMap<string, i32>,
}
"#;

    let input_dir = "tests/generated/non_sibling_test";
    fs::create_dir_all(input_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test_non_sibling.wj", input_dir);
    fs::write(&input_file, source).expect("Failed to write source file");

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

    let generated = fs::read_to_string(format!("{}/test_non_sibling.rs", input_dir))
        .expect("Failed to read output");

    println!("Generated code:\n{}", generated);

    // Non-sibling imports should remain as-is
    assert!(
        generated.contains("use std::collections::HashMap;"),
        "Expected 'use std::collections::HashMap;' unchanged but got:\n{}",
        generated
    );

    assert!(
        generated.contains("use some_external_crate::Thing;"),
        "Expected 'use some_external_crate::Thing;' unchanged but got:\n{}",
        generated
    );

    fs::remove_dir_all(input_dir).ok();
}
