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

//! WDB-359: indexed field access must not clone the whole element first.
//!
//! Product tip game-core `voxel/chunk_manager.rs`:
//!   `result.push(self.chunks[i].clone().coord)` with `ChunkCoord: Copy`
//! Prefer `self.chunks[i].coord` (Copy field). Distinct from WDB-343/350 (cast clone).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Coord {
    pub x: i32,
}

pub struct Chunk {
    pub coord: Coord,
    pub dirty: bool,
}

pub struct Manager {
    pub chunks: Vec<Chunk>,
}

pub fn dirty_coords(m: Manager) -> Vec<Coord> {
    let mut result: Vec<Coord> = Vec::new()
    let mut i: usize = 0
    while i < m.chunks.len() {
        if m.chunks[i].dirty {
            result.push(m.chunks[i].coord)
        }
        i = i + 1
    }
    result
}
"#;

#[test]
fn wdb359_module_file_index_field_must_not_clone_element() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-359 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-359 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone()") || rs.contains("].clone().");
    assert!(
        !bad,
        "WDB-359 RED: indexed access cloned whole element:\n{rs}"
    );
    test.cargo_check().expect("WDB-359 cargo-check");
}

#[test]
fn wdb359_tip_out_game_core_chunk_manager_must_not_clone_before_coord() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("chunk_manager.rs"),
        tip.join("voxel/chunk_manager.rs"),
        game.join("gen/voxel/chunk_manager.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("chunk_manager");
        let bad = text.lines().any(|line| {
            (line.contains("].clone().coord") || line.contains("].clone().grid"))
                && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-359: game-core/tip chunk_manager missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-359 RED: tip/product ].clone().field in:\n  {}",
        bad_paths.join("\n  ")
    );
}
