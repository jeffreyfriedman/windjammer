// TDD Test: Multi-file projects should NOT inline module definitions
// THE WINDJAMMER WAY: Use imports, not embedded modules!

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_multi_file_no_inline_modules() {
    // Create temporary directory with multi-file project
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    // Create math.wj
    fs::write(
        src_dir.join("math.wj"),
        r#"
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }
}
"#,
    )
    .unwrap();

    // Create physics.wj that imports math
    fs::write(
        src_dir.join("physics.wj"),
        r#"
use math::Vec2

pub struct PhysicsBody {
    pub position: Vec2,
}
"#,
    )
    .unwrap();

    // Compile project
    let output_dir = temp_dir.path().join("build");
    let status = std::process::Command::new("wj")
        .args(&[
            "build",
            src_dir.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .status()
        .unwrap();

    assert!(status.success(), "Compilation failed");

    // Read generated physics.rs
    let physics_rs = fs::read_to_string(output_dir.join("physics.rs")).unwrap();

    // ASSERT: Should NOT contain "pub mod math {"
    // The module system handles this with lib.rs and imports
    assert!(
        !physics_rs.contains("pub mod math {"),
        "Generated code should not inline module definitions!\nGenerated code:\n{}",
        physics_rs
    );

    // ASSERT: Should contain a proper import
    // Either "use crate::math::Vec2;" or "use super::math::Vec2;"
    assert!(
        physics_rs.contains("use crate::math::Vec2") || physics_rs.contains("use super::math"),
        "Generated code should have proper imports!\nGenerated code:\n{}",
        physics_rs
    );

    // ASSERT: Should contain PhysicsBody struct
    assert!(
        physics_rs.contains("pub struct PhysicsBody"),
        "Generated code should contain the actual struct!\nGenerated code:\n{}",
        physics_rs
    );
}
