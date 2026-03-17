// TDD: Test float literal inference for struct field assignments
//
// Bug: Float literals assigned to f32 fields generate _f64 suffix
//
// Example from breach-protocol:
//   struct Companion { pub attack_damage: f32 }
//   companion.attack_damage = 40.0  // Generates 40.0_f64, expects f32
//
// Root Cause: Float inference doesn't propagate from assignment target type

use tempfile::TempDir;
use windjammer::{build_project, CompilationTarget};

#[test]
fn test_float_literal_infers_from_field_type() {
    // Reproduce: assigning float literal to f32 field should infer f32
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("test.wj");
    let build = temp.path().join("build");
    
    std::fs::write(
        &src,
        r#"
pub struct Companion {
    pub attack_damage: f32,
    pub move_speed: f32
}

pub fn create_companion() -> Companion {
    let mut companion = Companion {
        attack_damage: 0.0,
        move_speed: 0.0
    }
    
    // These should infer f32 from field types
    companion.attack_damage = 40.0
    companion.move_speed = 3.5
    
    companion
}
"#,
    )
    .unwrap();
    
    build_project(&src, &build, CompilationTarget::Rust, false).expect("Build should succeed");
    
    let rust_code = std::fs::read_to_string(build.join("test.rs")).unwrap();
    
    // ASSERT: Should generate f32 suffixes, NOT f64
    assert!(
        !rust_code.contains("40.0_f64"),
        "Should NOT generate f64 suffix for f32 field. Found:\n{}",
        rust_code.lines().find(|l| l.contains("attack_damage") && l.contains("40")).unwrap_or("NOT FOUND")
    );
    
    assert!(
        !rust_code.contains("3.5_f64"),
        "Should NOT generate f64 suffix for f32 field. Found:\n{}",
        rust_code.lines().find(|l| l.contains("move_speed") && l.contains("3.5")).unwrap_or("NOT FOUND")
    );
    
    // Should have f32 suffixes
    assert!(
        rust_code.contains("40.0_f32") || rust_code.contains("40.0f32") || !rust_code.contains("_f64"),
        "Should use f32 for field assignment"
    );
}

#[test]
fn test_float_literal_in_arithmetic_with_field() {
    // Reproduce: arithmetic with f32 field should infer operands as f32
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("test.wj");
    let build = temp.path().join("build");
    
    std::fs::write(
        &src,
        r#"
pub struct Position {
    pub x: f32,
    pub y: f32
}

pub fn update_position(pos: Position, wave: i32) -> Position {
    let mut result = pos
    
    // These should infer f32 because result.x is f32
    result.x = 10.0 * (wave as f32)
    result.y = 5.0 * (wave as f32)
    
    result
}
"#,
    )
    .unwrap();
    
    build_project(&src, &build, CompilationTarget::Rust, false).expect("Build should succeed");
    
    let rust_code = std::fs::read_to_string(build.join("test.rs")).unwrap();
    
    // ASSERT: 10.0 and 5.0 should be f32, not f64
    assert!(
        !rust_code.contains("10.0_f64"),
        "Should NOT generate f64 in arithmetic with f32 field. Found:\n{}",
        rust_code.lines().find(|l| l.contains("10.0")).unwrap_or("NOT FOUND")
    );
    
    assert!(
        !rust_code.contains("5.0_f64"),
        "Should NOT generate f64 in arithmetic with f32 field"
    );
}

#[test]
fn test_float_literal_struct_constructor() {
    // Struct constructor literals should infer from field types
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("test.wj");
    let build = temp.path().join("build");
    
    std::fs::write(
        &src,
        r#"
pub struct Stats {
    pub health: f32,
    pub damage: f32
}

pub fn create_stats() -> Stats {
    Stats {
        health: 100.0,
        damage: 25.5
    }
}
"#,
    )
    .unwrap();
    
    build_project(&src, &build, CompilationTarget::Rust, false).expect("Build should succeed");
    
    let rust_code = std::fs::read_to_string(build.join("test.rs")).unwrap();
    
    // ASSERT: Constructor literals should infer from field types
    assert!(
        !rust_code.contains("100.0_f64"),
        "Constructor literal should infer f32 from field type"
    );
    
    assert!(
        !rust_code.contains("25.5_f64"),
        "Constructor literal should infer f32 from field type"
    );
}

#[test]
fn test_float_literal_method_return() {
    // Return values should infer from function signature
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("test.wj");
    let build = temp.path().join("build");
    
    std::fs::write(
        &src,
        r#"
pub fn get_speed() -> f32 {
    3.5
}

pub fn get_damage() -> f32 {
    40.0
}
"#,
    )
    .unwrap();
    
    build_project(&src, &build, CompilationTarget::Rust, false).expect("Build should succeed");
    
    let rust_code = std::fs::read_to_string(build.join("test.rs")).unwrap();
    
    // ASSERT: Return values should infer from signature
    assert!(
        !rust_code.contains("3.5_f64"),
        "Return value should infer f32 from function signature"
    );
    
    assert!(
        !rust_code.contains("40.0_f64"),
        "Return value should infer f32 from function signature"
    );
}
