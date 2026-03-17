// TDD: Test float literal inference for function arguments
//
// Bug: Float literals passed to f32 parameters generate _f64 suffix
//
// Example from breach-protocol:
//   fn new(health: f32, damage: f32) -> Stats { ... }
//   Stats::new(100.0, 50.0)  // Generates 100.0_f64, 50.0_f64 - WRONG!
//
// Root Cause: Float inference doesn't propagate from function parameter types

use tempfile::TempDir;
use windjammer::{build_project, CompilationTarget};

#[test]
fn test_float_literal_infers_from_function_params() {
    // CRITICAL: This should FAIL and expose the bug
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("test.wj");
    let build = temp.path().join("build");
    
    std::fs::write(
        &src,
        r#"
pub struct CombatStats {
    pub health: f32,
    pub damage: f32,
    pub defense: f32
}

impl CombatStats {
    pub fn new(health: f32, damage: f32, defense: f32) -> CombatStats {
        CombatStats { health, damage, defense }
    }
}

pub fn create_stats() -> CombatStats {
    // These literals should infer f32 from function parameters
    CombatStats::new(100.0, 50.0, 10.0)
}
"#,
    )
    .unwrap();
    
    build_project(&src, &build, CompilationTarget::Rust, false).expect("Build should succeed");
    
    let rust_code = std::fs::read_to_string(build.join("test.rs")).unwrap();
    
    println!("Generated code:\n{}", rust_code);
    
    // ASSERT: Should NOT generate f64 suffixes for f32 parameters
    assert!(
        !rust_code.contains("100.0_f64"),
        "Should infer f32 from parameter type, not f64. Generated:\n{}",
        rust_code.lines().filter(|l| l.contains("100.0") || l.contains("new")).collect::<Vec<_>>().join("\n")
    );
    
    assert!(
        !rust_code.contains("50.0_f64"),
        "Should infer f32 from parameter type"
    );
    
    assert!(
        !rust_code.contains("10.0_f64"),
        "Should infer f32 from parameter type"
    );
}

#[test]
fn test_float_literal_in_binary_expression() {
    // Test arithmetic expressions with type context
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("test.wj");
    let build = temp.path().join("build");
    
    std::fs::write(
        &src,
        r#"
pub fn calculate(wave: i32) -> f32 {
    10.0 * (wave as f32)
}

pub fn multiply_floats() -> f32 {
    3.5 * 2.0
}
"#,
    )
    .unwrap();
    
    build_project(&src, &build, CompilationTarget::Rust, false).expect("Build should succeed");
    
    let rust_code = std::fs::read_to_string(build.join("test.rs")).unwrap();
    
    // Return type is f32, so operands should be f32
    assert!(
        !rust_code.contains("10.0_f64"),
        "Operands in expression returning f32 should be f32"
    );
    
    assert!(
        !rust_code.contains("3.5_f64") && !rust_code.contains("2.0_f64"),
        "Float literals in expressions should infer from context"
    );
}
