#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

/// TDD Test: Doc Comments in Enums and Structs
///
/// This test verifies that doc comments (///) work properly in:
/// - Enum variant definitions
/// - Struct field definitions
/// - Function/method definitions (already working)
#[path = "../common/test_utils.rs"]
mod test_utils;

use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_doc_comments_on_enum_variants() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let _output_dir = temp_dir.path().to_path_buf();

    let code = r#"
/// A mode enumeration
pub enum Mode {
    /// Fast mode - highest performance
    Fast,
    /// Slow mode - lowest performance
    Slow,
    /// Medium mode - balanced
    Medium,
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Doc comments on enum variants should compile successfully. Error: {:?}",
        result.err()
    );

    let generated = result.unwrap();

    // Verify doc comments are preserved in generated Rust
    assert!(
        generated.contains("/// Fast mode"),
        "Doc comment for Fast variant should be preserved"
    );
    assert!(
        generated.contains("/// Slow mode"),
        "Doc comment for Slow variant should be preserved"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_doc_comments_on_struct_fields() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let _output_dir = temp_dir.path().to_path_buf();

    let code = r#"
/// A player structure
pub struct Player {
    /// Player's X position in world space
    pub x: f32,
    /// Player's Y position in world space
    pub y: f32,
    /// Player's health (0-100)
    pub health: i64,
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Doc comments on struct fields should compile successfully. Error: {:?}",
        result.err()
    );

    let generated = result.unwrap();

    // Verify doc comments are preserved in generated Rust
    assert!(
        generated.contains("/// Player's X position"),
        "Doc comment for x field should be preserved"
    );
    assert!(
        generated.contains("/// Player's health"),
        "Doc comment for health field should be preserved"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_doc_comments_on_functions() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let _output_dir = temp_dir.path().to_path_buf();

    let code = r#"
/// Calculate the sum of two numbers
pub fn add(a: i64, b: i64) -> i64 {
    a + b
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Doc comments on functions should compile successfully. Error: {:?}",
        result.err()
    );

    let generated = result.unwrap();

    // Verify doc comments are preserved
    assert!(
        generated.contains("/// Calculate the sum"),
        "Doc comment for function should be preserved"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mixed_doc_comments_and_regular_comments() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let _output_dir = temp_dir.path().to_path_buf();

    let code = r#"
/// Camera follow modes
pub enum CameraFollowMode {
    /// Camera instantly snaps to target position
    Instant,
    /// Camera smoothly lerps to target position
    Smooth,
    // Implementation note: Deadzone requires half_width calculation
    /// Camera follows with a deadzone (only moves when target leaves zone)
    Deadzone,
}

/// 2D Camera for games
pub struct Camera2D {
    /// Camera position in world space
    pub position: f32,
    // Internal: zoom factor
    /// Camera zoom (1.0 = normal, 2.0 = 2x zoom)
    pub zoom: f32,
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Mixed doc comments and regular comments should compile. Error: {:?}",
        result.err()
    );
}
