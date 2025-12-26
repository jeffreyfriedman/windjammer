/// TDD Test: Single nested module files should compile to correct subdirectories
///
/// BUG: When compiling a single nested file like `src_wj/ecs/entity.wj`,
/// the generated Rust code is written to `entity.rs` at root instead of `ecs/entity.rs`.
/// This happens when compiling individual files, not full projects.
///
/// THE WINDJAMMER WAY: Even individual file compilations should respect directory structure!
/// - `src_wj/ecs/entity.wj` -> `output/ecs/entity.rs`
/// - NOT `output/entity.rs`
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[ignore] // TODO: Fix compiler bug - single nested files should preserve directory structure
fn test_single_nested_file_compiles_to_correct_path() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path();

    // Create nested source structure
    let src_wj = project_root.join("src_wj");
    let ecs_dir = src_wj.join("ecs");
    std::fs::create_dir_all(&ecs_dir).unwrap();

    // Create a single nested file (simulating compiling one file at a time)
    std::fs::write(
        ecs_dir.join("entity.wj"),
        r#"
    pub struct Entity {
        pub id: i64,
        pub active: bool,
    }
    
    impl Entity {
        pub fn new(id: i64) -> Entity {
            Entity { id, active: true }
        }
    }
    "#,
    )
    .unwrap();

    // Compile just this single file (NOT a full project)
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let output_dir = project_root.join("output");

    let compile_result = Command::new(&wj_binary)
        .args([
            "build",
            ecs_dir.join("entity.wj").to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    assert!(
        compile_result.status.success(),
        "Compilation failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&compile_result.stdout),
        String::from_utf8_lossy(&compile_result.stderr)
    );

    // CRITICAL ASSERTION 1: ecs/entity.rs should exist and be non-empty
    let ecs_entity_rs = output_dir.join("ecs/entity.rs");
    assert!(
        ecs_entity_rs.exists(),
        "ecs/entity.rs should exist at: {}",
        ecs_entity_rs.display()
    );

    let entity_content =
        std::fs::read_to_string(&ecs_entity_rs).expect("Failed to read ecs/entity.rs");

    assert!(
        !entity_content.is_empty(),
        "ecs/entity.rs should NOT be empty! Got 0 bytes"
    );

    assert!(
        entity_content.contains("pub struct Entity"),
        "ecs/entity.rs should contain Entity struct, got:\n{}",
        entity_content
    );

    // CRITICAL ASSERTION 2: entity.rs should NOT exist at root
    let root_entity_rs = output_dir.join("entity.rs");
    assert!(
        !root_entity_rs.exists(),
        "entity.rs should NOT exist at root (should be in ecs/ subdirectory)"
    );
}

#[test]
fn test_full_project_nested_modules_compile_to_correct_paths() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path();

    // Create a nested Windjammer project structure
    let src_wj = project_root.join("src_wj");
    std::fs::create_dir_all(&src_wj).unwrap();

    // Create root mod.wj
    std::fs::write(
        src_wj.join("mod.wj"),
        r#"
    pub mod effects;
    "#,
    )
    .unwrap();

    // Create effects directory
    let effects_dir = src_wj.join("effects");
    std::fs::create_dir_all(&effects_dir).unwrap();

    // Create effects/mod.wj
    std::fs::write(
        effects_dir.join("mod.wj"),
        r#"
    pub use particle::Particle;
    pub use particle::ParticleEmitter;
    "#,
    )
    .unwrap();

    // Create effects/particle.wj
    std::fs::write(
        effects_dir.join("particle.wj"),
        r#"
    pub struct Particle {
        pub x: f32,
        pub y: f32,
    }
    
    pub struct ParticleEmitter {
        pub count: i64,
    }
    
    impl ParticleEmitter {
        pub fn new() -> ParticleEmitter {
            ParticleEmitter { count: 0 }
        }
        
        pub fn emit(self) {
            // Do nothing
        }
    }
    "#,
    )
    .unwrap();

    // Compile the project
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let output_dir = project_root.join("output");

    let compile_result = Command::new(&wj_binary)
        .args([
            "build",
            src_wj.join("mod.wj").to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--library",
            "--module-file",
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    assert!(
        compile_result.status.success(),
        "Compilation failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&compile_result.stdout),
        String::from_utf8_lossy(&compile_result.stderr)
    );

    // CRITICAL ASSERTION 1: effects/particle.rs should exist and be non-empty
    let effects_particle_rs = output_dir.join("effects/particle.rs");
    assert!(
        effects_particle_rs.exists(),
        "effects/particle.rs should exist at: {}",
        effects_particle_rs.display()
    );

    let particle_content =
        std::fs::read_to_string(&effects_particle_rs).expect("Failed to read effects/particle.rs");

    assert!(
        !particle_content.is_empty(),
        "effects/particle.rs should NOT be empty! Got 0 bytes"
    );

    assert!(
        particle_content.contains("pub struct Particle"),
        "effects/particle.rs should contain Particle struct, got:\n{}",
        particle_content
    );

    assert!(
        particle_content.contains("pub struct ParticleEmitter"),
        "effects/particle.rs should contain ParticleEmitter struct, got:\n{}",
        particle_content
    );

    // CRITICAL ASSERTION 2: particle.rs should NOT exist at root
    let root_particle_rs = output_dir.join("particle.rs");
    assert!(
        !root_particle_rs.exists(),
        "particle.rs should NOT exist at root (should be in effects/ subdirectory)"
    );

    // Verify it compiles with rustc
    let rustc_result = Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg(output_dir.join("lib.rs"))
        .arg("--out-dir")
        .arg(&output_dir)
        .arg("--edition")
        .arg("2021")
        .output()
        .expect("Failed to run rustc");

    assert!(
        rustc_result.status.success(),
        "Generated code should compile with rustc!\nrustc stderr:\n{}",
        String::from_utf8_lossy(&rustc_result.stderr)
    );
}
