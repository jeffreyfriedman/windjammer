#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Simulates a game crate calling engine `upload_svo(svo)` with metadata-loaded signatures.
#[test]
fn test_cross_crate_borrowed_vec_method_call_adds_ref() {
    let tmp = TempDir::new().expect("tempdir");

    // Engine crate
    let engine_src = tmp.path().join("engine_src");
    let engine_gen = tmp.path().join("engine_gen");
    let renderer_dir = engine_src.join("rendering");
    fs::create_dir_all(&renderer_dir).expect("mkdir");

    fs::write(
        renderer_dir.join("mod.wj"),
        r#"pub mod voxel;
"#,
    )
    .unwrap();

    fs::write(
        renderer_dir.join("voxel.wj"),
        r##"
pub struct VoxelGPURenderer {}

impl VoxelGPURenderer {
    pub fn upload_svo(self, svo_data: Vec<u32>, world_size: f32, depth: u32) {
        let _ = svo_data.len()
        let _ = world_size
        let _ = depth
    }
}
"##,
    )
    .unwrap();

    let engine_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            engine_src.to_str().unwrap(),
            "--output",
            engine_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("engine build");

    assert!(
        engine_build.status.success(),
        "engine library build failed:\n{}",
        String::from_utf8_lossy(&engine_build.stderr)
    );

    let metadata_path = engine_gen.join("metadata.json");
    assert!(
        metadata_path.exists(),
        "engine must emit metadata.json for cross-crate calls"
    );

    // Game crate consuming engine metadata
    let game_src = tmp.path().join("game_src");
    fs::create_dir_all(&game_src).expect("mkdir");

    fs::write(
        game_src.join("game.wj"),
        r##"
use engine::rendering::voxel::VoxelGPURenderer

pub fn upload_local_svo(renderer: VoxelGPURenderer) {
    let mut svo = Vec::new()
    svo.push(1u32)
    renderer.upload_svo(svo, 256.0, 5u32)
}
"##,
    )
    .unwrap();

    let game_gen = tmp.path().join("game_gen");
    let game_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            game_src.join("game.wj").to_str().unwrap(),
            "--output",
            game_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("engine={}", metadata_path.display()),
        ])
        .output()
        .expect("game build");

    assert!(
        game_build.status.success(),
        "game build failed:\n{}",
        String::from_utf8_lossy(&game_build.stderr)
    );

    let generated = fs::read_to_string(game_gen.join("game.rs")).expect("game.rs");

    assert!(
        generated.contains("upload_svo(&svo") || generated.contains("upload_svo(& svo"),
        "cross-crate call must borrow owned local Vec. Generated:\n{}",
        generated
    );
}
