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

/// Mirrors breach-protocol combat/damage.wj: static Vec3::new must not add & on f32 args
/// when engine metadata marks params Borrowed (stale) but param types are Copy f32.
fn build_engine(tmp: &TempDir) -> std::path::PathBuf {
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

    let status = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            engine_src.to_str().unwrap(),
            "--output",
            engine_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .status()
        .expect("engine build");
    assert!(status.success(), "engine library build failed");

    engine_gen.join("metadata.json")
}

#[test]
fn test_vec3_new_literals_and_fields_no_spurious_borrow() {
    let tmp = TempDir::new().expect("tempdir");
    let engine_meta = build_engine(&tmp);

    let game_src = tmp.path().join("game_src");
    fs::create_dir_all(&game_src).expect("mkdir");
    fs::write(
        game_src.join("damage.wj"),
        r##"
use engine::math::vec3::Vec3

pub fn miss_point() -> Vec3 {
    Vec3::new(0.0, 0.0, 0.0)
}

struct Enemy {
    x: f32,
    y: f32,
    z: f32,
}

pub fn enemy_position(enemy: Enemy) -> Vec3 {
    Vec3::new(enemy.x, enemy.y, enemy.z)
}

pub fn resolve_hitscan(origin: Vec3, direction: Vec3, max_range: f32) -> f32 {
    let zero = Vec3::new(0.0, 0.0, 0.0)
    origin.x + direction.y + max_range + zero.z
}
"##,
    )
    .unwrap();

    let game_gen = tmp.path().join("game_gen");
    let status = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            game_src.join("damage.wj").to_str().unwrap(),
            "--output",
            game_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("engine={}", engine_meta.display()),
        ])
        .status()
        .expect("game build");
    assert!(status.success(), "game build failed");

    let rs = fs::read_to_string(game_gen.join("damage.rs")).expect("damage.rs");

    assert!(
        rs.contains("Vec3::new(0.0_f32, 0.0_f32, 0.0_f32)")
            || rs.contains("Vec3::new(0.0, 0.0, 0.0)"),
        "literal Vec3::new must not borrow f32 args. Got:\n{rs}"
    );
    assert!(
        !rs.contains("Vec3::new(&0.0_f32") && !rs.contains("Vec3::new(&enemy."),
        "must not spuriously borrow Copy f32 Vec3::new args. Got:\n{rs}"
    );
    assert!(
        rs.contains("enemy.x, enemy.y, enemy.z"),
        "field args should pass by value. Got:\n{rs}"
    );
    assert!(
        rs.contains("origin: Vec3, direction: Vec3")
            && !rs.contains("origin: &Vec3"),
        "Copy Vec3 formals must be by-value. Got:\n{rs}"
    );
}
