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

//! WDB-334: owned Custom (`Vec3`) formal must not receive `&sample` (tps_camera).
//!
//! Product tip game-core `camera/tps_camera.rs`:
//!   `collides_point(grid.clone(), &sample, …)` with `pos: Vec3` → E0308.
//! Twin of WDB-331 (fps `&test_x`); distinct product surface (tps arm sample).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

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
fn wdb334_module_file_tps_owned_vec3_must_not_receive_ref_sample() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-334 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-334 MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("pos: Vec3") || rs.contains("mut pos: Vec3");
    assert!(owned, "WDB-334: expected owned Vec3 formal:\n{rs}");
    let bad = rs.contains("&sample") || rs.contains("collides_point(grid, &");
    assert!(
        !bad,
        "WDB-334 RED: owned Vec3 received &sample:\n{rs}"
    );
    test.cargo_check().expect("WDB-334 cargo-check");
}

#[test]
fn wdb334_tip_out_game_core_tps_camera_must_not_pass_ref_sample() {
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
        let bad = text.lines().any(|line| {
            line.contains("collides_point(") && line.contains("&sample")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-334: game-core/tip tps_camera missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-334 RED: tip/product passes &sample into owned Vec3 in:\n  {}",
        bad_paths.join("\n  ")
    );
}
