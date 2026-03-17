// TDD: Test cross-file float literal inference
//
// Bug: Float literals in function calls don't infer from cross-file signatures
//
// Example from breach-protocol:
//   File 1: combat_system.wj
//     impl CombatStats { pub fn new(health: f32, damage: f32) -> CombatStats }
//   
//   File 2: enemy.wj  
//     CombatStats::new(100.0, 50.0)  // Should infer f32, generates f64
//
// Root Cause: FloatInference only sees current file's function signatures

use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_cross_file_function_arg_inference() {
    // Reproduce exact breach-protocol pattern
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(src.join("combat")).unwrap();
    
    // File 1: Define struct and constructor with f32 params
    std::fs::write(
        src.join("combat/combat_system.wj"),
        r#"
pub struct CombatStats {
    pub health: f32,
    pub max_health: f32,
    pub damage: f32
}

impl CombatStats {
    pub fn new(max_health: f32, max_damage: f32, armor: f32) -> CombatStats {
        CombatStats {
            health: max_health,
            max_health: max_health,
            damage: max_damage
        }
    }
}
"#,
    )
    .unwrap();
    
    // File 2: Call constructor with bare float literals
    std::fs::write(
        src.join("combat/enemy.wj"),
        r#"
use crate::combat_system::CombatStats

pub struct Enemy {
    pub stats: CombatStats,
    pub x: f32,
    pub y: f32
}

pub fn create_grunt(x: f32, y: f32) -> Enemy {
    // These should infer f32 from CombatStats::new signature
    Enemy {
        stats: CombatStats::new(100.0, 50.0, 10.0),
        x: x,
        y: y
    }
}
"#,
    )
    .unwrap();
    
    // File 3: combat/mod.wj
    std::fs::write(
        src.join("combat/mod.wj"),
        r#"
pub mod combat_system
pub mod enemy
"#,
    )
    .unwrap();
    
    // File 4: root mod.wj
    std::fs::write(
        src.join("mod.wj"),
        r#"
pub mod combat
"#,
    )
    .unwrap();
    
    // Build as library
    build_project_ext(
        &src.join("mod.wj"),
        &build,
        CompilationTarget::Rust,
        false,
        true, // library mode
        &[],
    )
    .expect("Build should succeed");
    
    // Check enemy.rs - THIS IS WHERE THE BUG IS
    let enemy_code = std::fs::read_to_string(build.join("combat/enemy.rs")).unwrap();
    
    println!("Generated enemy.rs:\n{}", 
        enemy_code.lines()
            .filter(|l| l.contains("CombatStats::new") || l.contains("100.0") || l.contains("50.0"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    
    // ASSERT: Should generate f32 suffixes, NOT f64
    assert!(
        !enemy_code.contains("100.0_f64"),
        "Should infer f32 from CombatStats::new params (cross-file). Found:\n{}",
        enemy_code.lines().find(|l| l.contains("100.0")).unwrap_or("NOT FOUND")
    );
    
    assert!(
        !enemy_code.contains("50.0_f64"),
        "Should infer f32 from function signature (cross-file)"
    );
    
    assert!(
        !enemy_code.contains("10.0_f64"),
        "Should infer f32 from function signature (cross-file)"
    );
}

#[test]
fn test_cross_file_struct_field_inference() {
    // Test struct field assignment across files
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();
    
    // File 1: Define struct
    std::fs::write(
        src.join("types.wj"),
        r#"
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32
}
"#,
    )
    .unwrap();
    
    // File 2: Use struct
    std::fs::write(
        src.join("spawner.wj"),
        r#"
use crate::types::Position

pub fn spawn_at(x_pos: f32, y_pos: f32) -> Position {
    Position {
        x: x_pos,
        y: y_pos,
        z: 0.0
    }
}
"#,
    )
    .unwrap();
    
    // Root
    std::fs::write(
        src.join("mod.wj"),
        r#"
pub mod types
pub mod spawner
"#,
    )
    .unwrap();
    
    build_project_ext(
        &src.join("mod.wj"),
        &build,
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("Build should succeed");
    
    let spawner_code = std::fs::read_to_string(build.join("spawner.rs")).unwrap();
    
    // DEBUG: Print generated code
    eprintln!("=== Generated spawner.rs ===");
    eprintln!("{}", spawner_code);
    eprintln!("=========================");
    
    // ASSERT: z: 0.0 should infer f32 from Position struct (cross-file)
    assert!(
        !spawner_code.contains("0.0_f64"),
        "Struct field literal should infer f32 from field type (cross-file)"
    );
}
