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

/// Local bindings calling unique read-only methods (e.g. `palette.log_palette_summary()`)
/// must not infer MutBorrowed for cross-crate upload calls.
#[test]
fn test_local_palette_readonly_methods_do_not_force_mut_borrow_at_call() {
    let tmp = TempDir::new().expect("tempdir");
    let engine_src = tmp.path().join("engine_src");
    let mat_dir = engine_src.join("voxel");
    fs::create_dir_all(&mat_dir).expect("mkdir");

    fs::write(
        mat_dir.join("mod.wj"),
        r#"pub mod material;
"#,
    )
    .unwrap();

    fs::write(
        mat_dir.join("material.wj"),
        r##"
pub struct MaterialPalette {
    pub slot: f32,
}

impl MaterialPalette {
    pub fn log_palette_summary(self) {
        let _ = self.slot
    }

    pub fn to_gpu_buffer(self) -> Vec<f32> {
        let mut out = Vec::new()
        out.push(self.slot)
        out
    }
}
"##,
    )
    .unwrap();

    let engine_gen = tmp.path().join("engine_gen");
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

    let game_src = tmp.path().join("game_src");
    fs::create_dir_all(&game_src).expect("mkdir");

    fs::write(
        game_src.join("game.wj"),
        r##"
use engine::voxel::material::MaterialPalette

pub struct Renderer {}

impl Renderer {
    pub fn upload_material_palette(self, palette: MaterialPalette) {
        let _ = palette.to_gpu_buffer()
    }
}

pub fn init(renderer: Renderer) {
    let palette = MaterialPalette { slot: 1.0 }
    palette.log_palette_summary()
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
        "readonly local methods must not force &mut palette at upload call. Generated:\n{}",
        generated
    );
}
