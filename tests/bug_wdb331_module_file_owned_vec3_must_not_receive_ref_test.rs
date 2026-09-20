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

//! WDB-331: owned Custom (`Vec3`) formal must not receive `&test_x`.
//!
//! Product tip game-core `camera/fps_camera.rs`:
//!   `collides_aabb(&grid, &test_x, …)` with `pos: Vec3` → E0308.
//! Source WJ: `collides_aabb(grid, test_x, …)`. Twin of owned-String `&arg` (WDB-306).

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

pub fn collides_aabb(grid: Grid, pos: Vec3, scale: i32) -> bool {
    grid.ok && scale > 0 && (pos.x + pos.y + pos.z) > 0.0
}

pub fn try_move(grid: Grid, test_x: Vec3, scale: i32) -> bool {
    collides_aabb(grid, test_x, scale)
}
"#;

#[test]
fn wdb331_module_file_owned_vec3_must_not_receive_ref() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-331 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-331 MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("pos: Vec3") || rs.contains("mut pos: Vec3");
    assert!(owned, "WDB-331: expected owned Vec3 formal:\n{rs}");
    let bad = rs.contains("&test_x") || rs.contains("collides_aabb(grid, &");
    assert!(
        !bad,
        "WDB-331 RED: owned Vec3 received &test_x:\n{rs}"
    );
    test.cargo_check().expect("WDB-331 cargo-check");
}

#[test]
fn wdb331_tip_out_game_core_fps_camera_must_not_pass_ref_vec3() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("fps_camera.rs"),
        tip.join("camera/fps_camera.rs"),
        game.join("gen/camera/fps_camera.rs"),
        game.join("camera/fps_camera.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("fps_camera");
        let bad = text.lines().any(|line| {
            line.contains("collides_aabb(")
                && (line.contains("&test_x")
                    || line.contains("&test_y")
                    || line.contains("&test_z"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-331: game-core/tip fps_camera missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-331 RED: tip/product passes &test_* into owned Vec3 in:\n  {}",
        bad_paths.join("\n  ")
    );
}
