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

//! WDB-346: Copy `f32` field access must not emit `.x.clone()` / `.y.clone()` / `.z.clone()`.
//!
//! Product tip game-core `camera/tps_camera.rs` / `fps_camera.rs`:
//!   `Vec3::new(self.pivot.x.clone() + dx, self.pivot.y.clone(), …)`
//!   `pos.x = pos.x.clone() + push_x`
//! Prefer bare Copy field reads (no `.clone()`). Distinct from WDB-344 (locals).

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

pub struct Cam {
    pub pivot: Vec3,
}

pub fn sample(cam: Cam, dx: f32) -> Vec3 {
    Vec3 {
        x: cam.pivot.x + dx,
        y: cam.pivot.y,
        z: cam.pivot.z,
    }
}
"#;

#[test]
fn wdb346_module_file_copy_f32_field_must_not_emit_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-346 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-346 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()");
    assert!(
        !bad,
        "WDB-346 RED: Copy f32 field access emitted .clone():\n{rs}"
    );
    test.cargo_check().expect("WDB-346 cargo-check");
}

#[test]
fn wdb346_tip_out_game_core_camera_must_not_clone_f32_fields() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("tps_camera.rs"),
        tip.join("camera/tps_camera.rs"),
        tip.join("fps_camera.rs"),
        tip.join("camera/fps_camera.rs"),
        game.join("gen/camera/tps_camera.rs"),
        game.join("gen/camera/fps_camera.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("camera");
        let bad = text.lines().any(|line| {
            (line.contains(".x.clone()")
                || line.contains(".y.clone()")
                || line.contains(".z.clone()"))
                && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-346: game-core/tip camera files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-346 RED: tip/product clones Copy f32 fields in:\n  {}",
        bad_paths.join("\n  ")
    );
}
