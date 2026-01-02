/// TDD Test: Module System Imports
///
/// Reproduces E0432 errors where generated mod.rs files have incorrect imports.
///
/// The issue: When generating mod.rs for a directory, imports use `super::`
/// which doesn't work for sibling modules in the same directory.
///
/// Example structure:
///   ecs/
///     entity.wj      (defines Entity)
///     world.wj       (uses Entity)
///     mod.wj         (or auto-generated mod.rs)
///
/// Generated world.rs incorrectly has: `use super::entity::Entity;`
/// Should be: `use crate::ecs::entity::Entity;` or just `use entity::Entity;`
use std::fs;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_module_imports_within_directory() {
    // Create a temporary directory for our test project
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src_wj");
    let ecs_dir = src_dir.join("ecs");
    fs::create_dir_all(&ecs_dir).unwrap();

    // Create entity.wj - defines Entity struct
    let entity_wj = r#"
pub struct Entity {
    pub id: u32,
}

impl Entity {
    pub fn new(id: u32) -> Entity {
        Entity { id }
    }
}
"#;
    fs::write(ecs_dir.join("entity.wj"), entity_wj).unwrap();

    // Create world.wj - uses Entity from sibling file
    let world_wj = r#"
use crate::ecs::entity::Entity

pub struct World {
    pub entities: Vec<Entity>,
}

impl World {
    pub fn new() -> World {
        World {
            entities: Vec::new(),
        }
    }
    
    pub fn spawn(self) -> Entity {
        Entity::new(self.entities.len() as u32)
    }
}
"#;
    fs::write(ecs_dir.join("world.wj"), world_wj).unwrap();

    // Create mod.wj for the ecs directory
    let mod_wj = r#"
pub mod entity
pub mod world
"#;
    fs::write(ecs_dir.join("mod.wj"), mod_wj).unwrap();

    // Compile the project
    let output_dir = temp_dir.path().join("build");
    let result =
        windjammer::build_project(&src_dir, &output_dir, windjammer::CompilationTarget::Rust);

    // Should compile successfully
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    // Check that mod.rs was generated correctly
    let mod_rs_path = output_dir.join("ecs").join("mod.rs");
    assert!(mod_rs_path.exists(), "mod.rs should be generated");

    let mod_rs_content = fs::read_to_string(&mod_rs_path).unwrap();
    println!("Generated mod.rs:\n{}", mod_rs_content);

    // mod.rs should have pub mod declarations
    assert!(
        mod_rs_content.contains("pub mod entity"),
        "Should declare entity module"
    );
    assert!(
        mod_rs_content.contains("pub mod world"),
        "Should declare world module"
    );

    // Check that world.rs was generated correctly
    let world_rs_path = output_dir.join("ecs").join("world.rs");
    let world_rs_content = fs::read_to_string(&world_rs_path).unwrap();
    println!("Generated world.rs:\n{}", world_rs_content);

    // The import should be correct (either crate:: or super::super:: but NOT super::)
    // Since entity.rs is a sibling, the import should work
    assert!(
        world_rs_content.contains("use crate::ecs::entity::Entity")
            || world_rs_content.contains("use super::entity::Entity")
                && mod_rs_content.contains("pub mod entity"),
        "Import should use crate:: path or super:: with pub mod declaration"
    );

    // Verify the generated Rust actually compiles
    let cargo_output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&output_dir)
        .output()
        .expect("Failed to run cargo build");

    if !cargo_output.status.success() {
        let stderr = String::from_utf8_lossy(&cargo_output.stderr);
        println!("Cargo build failed:\n{}", stderr);
        panic!("Generated Rust code should compile");
    }
}

#[test]
fn test_prelude_reexports() {
    // Test that a prelude.wj can re-export types from other modules
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src_wj");
    let math_dir = src_dir.join("math");
    fs::create_dir_all(&math_dir).unwrap();
    fs::create_dir_all(&src_dir).unwrap();

    // Create math/vec2.wj
    let vec2_wj = r#"
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}
"#;
    fs::write(math_dir.join("vec2.wj"), vec2_wj).unwrap();

    // Create math/mod.wj
    let math_mod_wj = r#"
pub mod vec2
"#;
    fs::write(math_dir.join("mod.wj"), math_mod_wj).unwrap();

    // Create prelude.wj that re-exports Vec2
    let prelude_wj = r#"
pub use crate::math::vec2::Vec2
"#;
    fs::write(src_dir.join("prelude.wj"), prelude_wj).unwrap();

    // Compile
    let output_dir = temp_dir.path().join("build");
    let result =
        windjammer::build_project(&src_dir, &output_dir, windjammer::CompilationTarget::Rust);
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    // Check prelude.rs has correct re-export
    let prelude_rs = fs::read_to_string(output_dir.join("prelude.rs")).unwrap();
    assert!(
        prelude_rs.contains("pub use crate::math::vec2::Vec2"),
        "Prelude should re-export Vec2"
    );

    // Verify cargo build works
    let cargo_output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&output_dir)
        .output()
        .expect("Failed to run cargo build");

    assert!(
        cargo_output.status.success(),
        "Generated Rust should compile. stderr: {}",
        String::from_utf8_lossy(&cargo_output.stderr)
    );
}
