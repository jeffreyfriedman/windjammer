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

//! WDB-385: Copy associated consts must not emit `.clone()`.
//!
//! Product tip game-core `voxel/voxel_scene.rs`:
//!   `Vec3::new(f32::MAX.clone(), f32::MAX.clone(), f32::MAX.clone())`
//! WJ source is `Vec3::new(f32::MAX, f32::MAX, f32::MAX)`. Prefer bare `f32::MAX`.

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

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
    }
}

pub fn bounds_min() -> Vec3 {
    Vec3::new(f32::MAX, f32::MAX, f32::MAX)
}

pub fn bounds_max() -> Vec3 {
    Vec3::new(f32::MIN, f32::MIN, f32::MIN)
}
"#;

#[test]
fn wdb385_module_file_f32_assoc_const_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-385 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-385 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("f32::MAX.clone()") || rs.contains("f32::MIN.clone()");
    assert!(
        !bad,
        "WDB-385 RED: f32 associated const cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-385 cargo-check");
}

#[test]
fn wdb385_tip_out_game_core_voxel_scene_must_not_clone_f32_max() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("voxel/voxel_scene.rs"),
        tip.join("voxel_scene.rs"),
        game.join("gen/voxel/voxel_scene.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            !line.trim_start().starts_with("//")
                && (line.contains("f32::MAX.clone()") || line.contains("f32::MIN.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-385: voxel_scene product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-385 RED: tip/product f32::MAX/MIN.clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
