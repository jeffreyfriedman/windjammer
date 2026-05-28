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

/// Game crates must not pass their own metadata.json when engine metadata is available.
/// Stale game metadata embeds wrong engine param_ownership and poisons cross-crate calls.
#[test]
fn test_stale_game_metadata_does_not_force_mut_palette_upload() {
    let tmp = TempDir::new().expect("tempdir");

    // Engine with read-only palette param
    let engine_src = tmp.path().join("engine_src");
    let engine_gen = tmp.path().join("engine_gen");
    let renderer_dir = engine_src.join("rendering");
    fs::create_dir_all(&renderer_dir).expect("mkdir");

    fs::write(
        renderer_dir.join("mod.wj"),
        r#"pub mod gpu;
"#,
    )
    .unwrap();

    fs::write(
        renderer_dir.join("gpu.wj"),
        r##"
pub struct MaterialPalette {
    pub slot: f32,
}

pub struct VoxelGPURenderer {}

impl VoxelGPURenderer {
    pub fn upload_material_palette(self, palette: MaterialPalette) {
        let _ = palette.slot
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
        "engine build failed:\n{}",
        String::from_utf8_lossy(&engine_build.stderr)
    );

    let engine_meta = fs::read_to_string(engine_gen.join("metadata.json")).expect("engine meta");
    assert!(
        engine_meta.contains("VoxelGPURenderer::upload_material_palette"),
        "engine metadata missing upload_material_palette"
    );
    assert!(
        engine_meta.contains("\"Borrowed\""),
        "engine metadata should include Borrowed palette param"
    );

    // Stale game metadata: wrong MutBorrowed for palette (simulates prior bad build output)
    let stale_game_meta = tmp.path().join("stale_game_metadata.json");
    let stale = engine_meta.replace(
        "\"VoxelGPURenderer::upload_material_palette\": {\n      \"params\": [\n        \"Custom(\\\"Self\\\")\",\n        \"Custom(\\\"MaterialPalette\\\")\"\n      ],\n      \"return_type\": null,\n      \"is_associated\": true,\n      \"parent_type\": \"VoxelGPURenderer\",\n      \"param_ownership\": [\n        \"MutBorrowed\",\n        \"Borrowed\"\n      ],",
        "\"VoxelGPURenderer::upload_material_palette\": {\n      \"params\": [\n        \"Custom(\\\"Self\\\")\",\n        \"Custom(\\\"MaterialPalette\\\")\"\n      ],\n      \"return_type\": null,\n      \"is_associated\": true,\n      \"parent_type\": \"VoxelGPURenderer\",\n      \"param_ownership\": [\n        \"MutBorrowed\",\n        \"MutBorrowed\"\n      ],",
    );
    fs::write(&stale_game_meta, stale).expect("write stale meta");

    let game_src = tmp.path().join("game_src");
    fs::create_dir_all(&game_src).expect("mkdir");

    fs::write(
        game_src.join("game.wj"),
        r##"
use engine::rendering::gpu::{MaterialPalette, VoxelGPURenderer}

pub fn init(renderer: VoxelGPURenderer) {
    let palette = MaterialPalette { slot: 1.0 }
    renderer.upload_material_palette(palette)
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
            &format!("engine={}", engine_gen.join("metadata.json").display()),
            "--metadata",
            &format!("game_core={}", stale_game_meta.display()),
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
        !generated.contains("&mut palette"),
        "fresh engine metadata must win over stale game metadata. Generated:\n{}",
        generated
    );
}
