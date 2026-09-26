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

//! WDB-392: Copy `Direction::PosX` must not emit `.clone()` (WDB-384 path miss).
//!
//! WDB-384 tip list covers FaceDirection / AlertLevel / NPCBehavior /
//! ChunkLifecycleState / MaterialNodeKind, but product
//! gen/voxel/meshing.rs still has `Direction::PosX.clone()`.
//! WJ source is `direction: Direction::PosX`. Twin of WDB-384.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum Direction {
    PosX,
    NegX,
}

pub struct VoxelFace {
    pub direction: Direction,
    pub color_id: u8,
}

pub fn east_face(color_id: u8) -> VoxelFace {
    VoxelFace {
        direction: Direction::PosX,
        color_id: color_id,
    }
}

pub fn flip(dir: Direction) -> Direction {
    Direction::NegX
}
"#;

#[test]
fn wdb392_module_file_direction_unit_variant_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-392 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-392 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("Direction::PosX.clone()") || rs.contains("Direction::NegX.clone()");
    assert!(
        !bad,
        "WDB-392 RED: Copy Direction unit variant cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-392 cargo-check");
}

#[test]
fn wdb392_tip_out_game_core_meshing_must_not_clone_direction() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("voxel/meshing.rs"),
        tip.join("meshing.rs"),
        game.join("gen/voxel/meshing.rs"),
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
                && line.contains("Direction::")
                && line.contains(".clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-392: meshing product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-392 RED: tip/product Direction::*.clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
