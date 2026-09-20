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

//! WDB-360: `encode(grid)` must not require `grid.clone()` when encode only reads.
//!
//! Product tip game-core cluster:
//!   `encoder.encode(self.grid.clone())` / `encode(self.chunks[i].clone().grid.clone())`
//!   with `encode(&mut self, grid: VoxelGrid)` owned formal forcing a clone of a large grid.
//! Prefer borrowed formal `encode(grid: &VoxelGrid)` + call `encode(&self.grid)`.
//! Twin polarity of WDB-335 (owned clone into demoted `&VoxelGrid`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct VoxelGrid {
    pub w: i32,
}

pub struct Encoder {
    pub n: i32,
}

impl Encoder {
    pub fn encode(self, grid: VoxelGrid) -> Vec<i32> {
        let mut out: Vec<i32> = Vec::new()
        out.push(grid.w)
        out
    }
}

pub struct Scene {
    pub grid: VoxelGrid,
}

pub fn bake(scene: Scene) -> Vec<i32> {
    let mut enc = Encoder { n: 0 }
    enc.encode(scene.grid)
}
"#;

#[test]
fn wdb360_module_file_encode_must_not_force_grid_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-360 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-360 MultiFile lib.rs:\n{rs}");
    // After fix: encode should take &VoxelGrid and call sites borrow — no .clone() on grid.
    let bad = rs.contains("grid.clone()") || rs.contains(".grid.clone()");
    assert!(
        !bad,
        "WDB-360 RED: encode path cloned grid:\n{rs}"
    );
    test.cargo_check().expect("WDB-360 cargo-check");
}

#[test]
fn wdb360_tip_out_game_core_encode_must_not_clone_grid() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("voxel_editor.rs"),
        tip.join("editor/voxel_editor.rs"),
        tip.join("voxel_scene.rs"),
        tip.join("quick_start/voxel_scene.rs"),
        tip.join("chunk_manager.rs"),
        tip.join("voxel/chunk_manager.rs"),
        tip.join("svo.rs"),
        tip.join("voxel/svo.rs"),
        game.join("gen/editor/voxel_editor.rs"),
        game.join("gen/quick_start/voxel_scene.rs"),
        game.join("gen/voxel/chunk_manager.rs"),
        game.join("gen/voxel/svo.rs"),
        game.join("gen/demos/cathedral.rs"),
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
            line.contains("encode(")
                && line.contains(".clone()")
                && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-360: game-core/tip encode call sites missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-360 RED: tip/product encode(…clone…) in:\n  {}",
        bad_paths.join("\n  ")
    );
}
