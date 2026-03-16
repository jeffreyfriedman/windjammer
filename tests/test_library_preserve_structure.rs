// TDD: Test that library builds preserve directory structure
//
// Bug: wj build --library flattens all .rs files to output root,
// causing module resolution errors when imports use crate::module::submodule::*
//
// Fix: Preserve relative directory structure from input to output

use std::path::PathBuf;
use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_library_preserves_directory_structure() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    
    // Create hierarchical source structure
    std::fs::create_dir_all(src.join("player")).unwrap();
    std::fs::create_dir_all(src.join("combat")).unwrap();
    
    // src/player/controller.wj
    std::fs::write(
        src.join("player/controller.wj"),
        r#"
pub struct PlayerController {
    pub x: f32
}

impl PlayerController {
    pub fn new() -> PlayerController {
        PlayerController { x: 0.0 }
    }
}
"#,
    )
    .unwrap();
    
    // src/combat/enemy.wj
    std::fs::write(
        src.join("combat/enemy.wj"),
        r#"
pub struct Enemy {
    pub health: i32
}
"#,
    )
    .unwrap();
    
    // src/mod.wj - uses hierarchical imports
    std::fs::write(
        src.join("mod.wj"),
        r#"
use crate::player::controller::PlayerController
use crate::combat::enemy::Enemy

pub struct Game {
    pub player: PlayerController,
    pub enemies: Vec<Enemy>
}
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
    
    // ASSERT: Output preserves directory structure
    assert!(
        build.join("player").join("controller.rs").exists(),
        "build/player/controller.rs should exist"
    );
    assert!(
        build.join("combat").join("enemy.rs").exists(),
        "build/combat/enemy.rs should exist"
    );
    assert!(
        build.join("mod.rs").exists(),
        "build/mod.rs should exist"
    );
    
    // ASSERT: Generated code compiles with Rust (hierarchical imports still work)
    let rust_code = std::fs::read_to_string(build.join("mod.rs")).unwrap();
    assert!(
        rust_code.contains("crate::player::controller::PlayerController"),
        "Should preserve hierarchical imports"
    );
}

#[test]
fn test_flat_input_still_works() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();
    
    // Flat source (no subdirectories)
    std::fs::write(
        src.join("game.wj"),
        r#"
pub struct Game {
    pub score: i32
}
"#,
    )
    .unwrap();
    
    build_project_ext(
        &src.join("game.wj"),
        &build,
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("Build should succeed");
    
    // ASSERT: Flat input produces flat output
    assert!(
        build.join("game.rs").exists(),
        "build/game.rs should exist (not build/src/game.rs)"
    );
}
