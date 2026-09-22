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

//! WDB-370: Copy field must not emit `].clone().coord.clone()` (double clone).
//!
//! Product tip game-core `world/streaming.rs`:
//!   `chunk_chebyshev_distance(self.chunks[i].clone().coord.clone(), pc.clone())`
//! Prefer `self.chunks[i].coord` (Copy `ChunkCoord`) or at most one clone of non-Copy element.
//! Twin of WDB-359 / WDB-363 index-field clone cluster.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Coord {
    pub x: i32,
    pub z: i32,
}

pub struct Chunk {
    pub coord: Coord,
}

pub fn chebyshev(a: Coord, b: Coord) -> i32 {
    let dx = a.x - b.x
    if dx < 0 {
        -dx
    } else {
        dx
    }
}

pub fn dist_from_chunk(chunks: Vec<Chunk>, i: usize, b: Coord) -> i32 {
    chebyshev(chunks[i].coord, b)
}
"#;

#[test]
fn wdb370_module_file_copy_field_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-370 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-370 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().coord") || rs.contains(".coord.clone()");
    assert!(
        !bad,
        "WDB-370 RED: indexed Copy field double-cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-370 cargo-check");
}

#[test]
fn wdb370_tip_out_game_core_streaming_must_not_double_clone_coord() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("world/streaming.rs"),
        tip.join("streaming.rs"),
        tip.join("voxel/chunk_manager.rs"),
        tip.join("chunk_manager.rs"),
        game.join("gen/world/streaming.rs"),
        game.join("gen/voxel/chunk_manager.rs"),
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
                && (line.contains("].clone().coord")
                    || line.contains("].clone().coord.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-370: streaming/chunk_manager product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-370 RED: tip/product indexed coord double-clone in:\n  {}",
        bad_paths.join("\n  ")
    );
}
