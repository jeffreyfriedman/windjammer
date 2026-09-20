#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
    feature = "codegen_tests",
))]

//! WDB-335: borrowed `&Grid` formal must not receive owned `grid.clone()`.
//!
//! Product tip game-core `camera/tps_camera.rs`:
//!   `collides_point(grid.clone(), &sample, …)` with `grid: &VoxelGrid` → E0308
//!   (expected `&VoxelGrid`, found `VoxelGrid`).
//! Opposite polarity of WDB-331 (owned formal + `&arg`). Prefer `&grid` when
//! formal is borrowed, or owned formal + move/`clone` consistently.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

// Force borrowed grid formal via read-only use (no field mutation).
const SRC: &str = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Grid {
    pub ok: bool,
}

pub fn collides_point(grid: Grid, pos: Vec3, scale: i32) -> bool {
    grid.ok && scale > 0 && (pos.x + pos.y + pos.z) > 0.0
}

pub fn sample_arm(grid: Grid, sample: Vec3, scale: i32) -> bool {
    collides_point(grid, sample, scale)
}
"#;

#[test]
fn wdb335_module_file_must_not_pass_owned_clone_into_borrowed_or_ref_sample() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-335 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-335 MultiFile lib.rs:\n{rs}");
    // Accept either owned Grid formal + owned arg, or &Grid + &grid — not mixed.
    let formal_borrowed = rs.contains("grid: &Grid") || rs.contains("grid: &mut Grid");
    let call_owned_clone = rs.contains("grid.clone()");
    let call_ref_sample = rs.contains("&sample");
    let bad_mix = formal_borrowed && call_owned_clone;
    let bad_ref = call_ref_sample;
    assert!(
        !bad_mix && !bad_ref,
        "WDB-335 RED: borrowed/owned polarity mismatch or &sample:\n{rs}"
    );
    test.cargo_check().expect("WDB-335 cargo-check");
}

#[test]
fn wdb335_tip_out_game_core_tps_must_not_pass_owned_clone_into_borrowed_grid() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("tps_camera.rs"),
        tip.join("camera/tps_camera.rs"),
        game.join("gen/camera/tps_camera.rs"),
        game.join("camera/tps_camera.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("tps_camera");
        let formal_borrowed = text.contains("grid: &VoxelGrid") || text.contains("grid: &Grid");
        let bad_call = text.lines().any(|line| {
            line.contains("collides_point(") && line.contains("grid.clone()")
        });
        if formal_borrowed && bad_call {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-335: game-core/tip tps_camera missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-335 RED: tip/product passes grid.clone() into &VoxelGrid in:\n  {}",
        bad_paths.join("\n  ")
    );
}
