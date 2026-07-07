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

/// Copy engine types (Vec3) must lower to by-value formal params even when
/// ownership inference marks them Borrowed — same as f32/i32 Copy scalars.
#[test]
fn test_copy_vec3_formal_params_not_ref_with_engine_metadata() {
    let tmp = TempDir::new().expect("tempdir");

    let engine_src = tmp.path().join("engine_src");
    let engine_gen = tmp.path().join("engine_gen");
    fs::create_dir_all(engine_src.join("math")).expect("mkdir");

    fs::write(engine_src.join("mod.wj"), "mod math\n").unwrap();
    fs::write(engine_src.join("math/mod.wj"), "mod vec3\n").unwrap();
    fs::write(
        engine_src.join("math/vec3.wj"),
        r##"
pub struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
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

    let engine_meta = engine_gen.join("metadata.json");
    assert!(engine_meta.exists(), "engine must emit metadata.json");

    let game_src = tmp.path().join("game_src");
    fs::create_dir_all(&game_src).expect("mkdir");
    fs::write(
        game_src.join("damage.wj"),
        r##"
use engine::math::vec3::Vec3

pub fn resolve_hitscan(origin: Vec3, direction: Vec3, max_range: f32) -> f32 {
    origin.x + direction.y + max_range
}
"##,
    )
    .unwrap();

    let game_gen = tmp.path().join("game_gen");
    let game_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            game_src.join("damage.wj").to_str().unwrap(),
            "--output",
            game_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("engine={}", engine_meta.display()),
        ])
        .output()
        .expect("game build");

    assert!(
        game_build.status.success(),
        "game build failed:\n{}",
        String::from_utf8_lossy(&game_build.stderr)
    );

    let rs = fs::read_to_string(game_gen.join("damage.rs")).expect("damage.rs");
    assert!(
        rs.contains("pub fn resolve_hitscan(origin: Vec3, direction: Vec3, max_range: f32)"),
        "Copy Vec3 params must be by-value, not &Vec3. Generated:\n{rs}"
    );
    assert!(
        !rs.contains("origin: &Vec3") && !rs.contains("direction: &Vec3"),
        "Must not borrow Copy Vec3 formal params. Generated:\n{rs}"
    );
}
