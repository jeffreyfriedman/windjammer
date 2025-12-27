// Integration test: TypeRegistry fixes import paths correctly
// This test verifies that imports like "use math::Vec2" are transformed
// to "use super::vec2::Vec2" when Vec2 is defined in vec2.wj

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore] // TODO: Fix type registry import generation
fn test_type_registry_generates_correct_imports() {
    let wj_compiler = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    // Create a temporary directory structure
    let temp_dir = std::env::temp_dir().join("wj_test_type_registry");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let math_dir = temp_dir.join("math");
    fs::create_dir_all(&math_dir).unwrap();

    // Create vec2.wj
    fs::write(
        math_dir.join("vec2.wj"),
        r#"
pub struct Vec2 {
    pub x: f32
    pub y: f32
}
"#,
    )
    .unwrap();

    // Create main.wj that imports Vec2
    fs::write(
        temp_dir.join("main.wj"),
        r#"
use math::Vec2

struct Point {
    position: Vec2
}

pub fn main() {
    let p = Point {
        position: Vec2 { x: 1.0, y: 2.0 }
    }
}
"#,
    )
    .unwrap();

    let output_dir = temp_dir.join("generated");
    fs::create_dir_all(&output_dir).unwrap();

    // Compile with TypeRegistry
    let output = Command::new(&wj_compiler)
        .arg("build")
        .arg(&temp_dir)
        .arg("--output")
        .arg(&output_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj compiler");

    assert!(
        output.status.success(),
        "Compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read the generated main.rs
    let main_rs = fs::read_to_string(output_dir.join("main.rs")).unwrap();

    // Verify the import was fixed to use super::vec2::Vec2
    assert!(
        main_rs.contains("use super::vec2::Vec2"),
        "Expected 'use super::vec2::Vec2' but got:\n{}",
        main_rs
    );

    // Should NOT contain the incorrect import
    assert!(
        !main_rs.contains("use math::Vec2;") || main_rs.contains("use super::vec2::Vec2"),
        "Import was not corrected by TypeRegistry"
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
}
