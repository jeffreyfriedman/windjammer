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

/// `palette.to_gpu_buffer()` must not infer MutBorrowed for read-only MaterialPalette params.
#[test]
fn test_to_gpu_buffer_does_not_infer_mut_borrow_on_palette() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    let voxel_dir = src.join("voxel");
    let render_dir = src.join("rendering");
    fs::create_dir_all(&voxel_dir).expect("mkdir");
    fs::create_dir_all(&render_dir).expect("mkdir");

    fs::write(
        voxel_dir.join("mod.wj"),
        r#"pub mod material;
"#,
    )
    .unwrap();

    fs::write(
        voxel_dir.join("material.wj"),
        r##"
pub struct MaterialPalette {
    pub slot: f32,
}

impl MaterialPalette {
    pub fn to_gpu_buffer(self) -> Vec<f32> {
        let mut out = Vec::new()
        out.push(self.slot)
        out
    }
}
"##,
    )
    .unwrap();

    fs::write(
        render_dir.join("mod.wj"),
        r#"pub mod gpu;
"#,
    )
    .unwrap();

    fs::write(
        render_dir.join("gpu.wj"),
        r##"
use crate::voxel::material::MaterialPalette

pub struct Renderer {}

impl Renderer {
    pub fn upload_material_palette(self, palette: MaterialPalette) {
        let _ = palette.to_gpu_buffer()
    }
}
"##,
    )
    .unwrap();

    let out = tmp.path().join("gen");
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");

    assert!(
        output.status.success(),
        "library build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let metadata = fs::read_to_string(out.join("metadata.json")).expect("metadata.json");
    assert!(
        metadata.contains("Renderer::upload_material_palette")
            || metadata.contains("upload_material_palette"),
        "metadata should list upload_material_palette. Got:\n{}",
        metadata
    );
    assert!(
        !metadata.contains("\"MutBorrowed\",\n        \"MutBorrowed\"\n      ],\n      \"has_self_receiver\": true,\n      \"is_extern\": false\n    },\n    \"MaterialPalette::to_gpu_buffer\""),
        "palette param must not be MutBorrowed after to_gpu_buffer readonly call"
    );

    // Cross-crate call site must use &, not &mut
    let game_src = tmp.path().join("game_src");
    fs::create_dir_all(&game_src).expect("mkdir");
    fs::write(
        game_src.join("game.wj"),
        r##"
use crate::rendering::gpu::Renderer
use crate::voxel::material::MaterialPalette

pub fn upload(renderer: Renderer) {
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
            &format!("engine={}", out.join("metadata.json").display()),
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
        "call site must not use &mut for read-only palette. Generated:\n{}",
        generated
    );
}
