/// TDD Test: Trait regeneration should not leak modules into mod.rs files
///
/// BUG: After trait inference and regeneration, mod.rs files are incorrectly
/// including inline definitions of modules from OTHER directories.
///
/// EXPECTED: ecs/mod.rs should ONLY declare local modules (entity, components, etc.)
/// NOT inline definitions of physics modules (rigidbody2d, collision2d).
use std::process::Command;

#[test]
fn test_trait_regen_does_not_leak_modules() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_root = temp_dir.path();
    let src_wj = project_root.join("src_wj");

    // Create a multi-file project with trait implementations
    std::fs::create_dir_all(&src_wj).unwrap();

    // Create root mod.wj
    std::fs::write(
        src_wj.join("mod.wj"),
        r#"
pub mod ecs;
pub mod physics;

pub trait MyTrait {
    fn do_something(self);
}
"#,
    )
    .unwrap();

    // Create ecs module with explicit module declarations
    let ecs_dir = src_wj.join("ecs");
    std::fs::create_dir_all(&ecs_dir).unwrap();
    std::fs::write(
        ecs_dir.join("mod.wj"),
        r#"
// ECS Module
pub mod entity
pub mod components

pub use entity::Entity
"#,
    )
    .unwrap();

    std::fs::write(
        ecs_dir.join("entity.wj"),
        r#"
use crate::MyTrait

pub struct Entity {
    pub id: i64,
}

impl MyTrait for Entity {
    fn do_something(self) {
        println!("Entity {}", self.id);
    }
}
"#,
    )
    .unwrap();

    std::fs::write(
        ecs_dir.join("components.wj"),
        r#"
pub struct Transform {
    pub x: f32,
    pub y: f32,
}
"#,
    )
    .unwrap();

    // Create physics module
    let physics_dir = src_wj.join("physics");
    std::fs::create_dir_all(&physics_dir).unwrap();
    std::fs::write(physics_dir.join("mod.wj"), "pub mod rigidbody;").unwrap();
    std::fs::write(
        physics_dir.join("rigidbody.wj"),
        r#"
use crate::MyTrait

pub struct RigidBody {
    pub mass: f32,
}

impl MyTrait for RigidBody {
    fn do_something(self) {
        println!("RigidBody mass={}", self.mass);
    }
}
"#,
    )
    .unwrap();

    // Compile the project
    let output_dir = project_root.join("out");
    let compile_result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(src_wj.join("mod.wj"))
        .arg("--output")
        .arg(&output_dir)
        .arg("--library")
        .arg("--no-cargo")
        .output()
        .expect("Failed to execute compiler");

    if !compile_result.status.success() {
        eprintln!(
            "STDOUT:\n{}",
            String::from_utf8_lossy(&compile_result.stdout)
        );
        eprintln!(
            "STDERR:\n{}",
            String::from_utf8_lossy(&compile_result.stderr)
        );
        panic!("Compiler failed");
    }

    // Read generated ecs/mod.rs
    let ecs_mod_rs =
        std::fs::read_to_string(output_dir.join("ecs/mod.rs")).expect("Failed to read ecs/mod.rs");

    println!(
        "=== Generated ecs/mod.rs ===\n{}\n========================",
        ecs_mod_rs
    );

    // CRITICAL: ecs/mod.rs should NOT contain RigidBody (that's in physics/)
    assert!(
        !ecs_mod_rs.contains("pub struct RigidBody"),
        "ecs/mod.rs should NOT contain RigidBody struct (belongs in physics/)!\nGenerated:\n{}",
        ecs_mod_rs
    );

    assert!(
        !ecs_mod_rs.contains("pub mod rigidbody {"),
        "ecs/mod.rs should NOT inline rigidbody module (belongs in physics/)!\nGenerated:\n{}",
        ecs_mod_rs
    );

    // ecs/mod.rs SHOULD declare the local entity module
    assert!(
        ecs_mod_rs.contains("pub mod entity;"),
        "ecs/mod.rs should declare entity module!\nGenerated:\n{}",
        ecs_mod_rs
    );

    // Similarly, physics/mod.rs should NOT contain Entity
    let physics_mod_rs = std::fs::read_to_string(output_dir.join("physics/mod.rs"))
        .expect("Failed to read physics/mod.rs");

    println!(
        "=== Generated physics/mod.rs ===\n{}\n========================",
        physics_mod_rs
    );

    assert!(
        !physics_mod_rs.contains("pub struct Entity"),
        "physics/mod.rs should NOT contain Entity struct (belongs in ecs/)!\nGenerated:\n{}",
        physics_mod_rs
    );

    assert!(
        physics_mod_rs.contains("pub mod rigidbody;"),
        "physics/mod.rs should declare rigidbody module!\nGenerated:\n{}",
        physics_mod_rs
    );
}
