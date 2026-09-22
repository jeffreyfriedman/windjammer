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

//! WDB-361: `usize` loop counters must not emit redundant `(i as usize)`.
//!
//! Product tip game-core cluster (chunk_manager, ecs, scenes, lod, …):
//!   `while (i as usize) < self.chunks.len()` with `i: usize`
//! Prefer bare `i < self.chunks.len()`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Manager {
    pub items: Vec<i32>,
}

pub fn count(m: Manager) -> usize {
    let mut i: usize = 0
    let mut n: usize = 0
    while i < m.items.len() {
        n = n + 1
        i = i + 1
    }
    n
}

pub fn find_in_order(m: Manager, target: i32) -> i32 {
    let mut i = 0
    while i < m.items.len() {
        if m.items[i] == target {
            return i
        }
        i = i + 1
    }
    -1
}
"#;

#[test]
fn wdb361_module_file_usize_counter_must_not_cast_as_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-361 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-361 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("i as usize") || rs.contains("(i as usize)");
    assert!(
        !bad,
        "WDB-361 RED: usize counter emitted as usize:\n{rs}"
    );
    test.cargo_check().expect("WDB-361 cargo-check");
}

#[test]
fn wdb361_tip_out_game_core_must_not_cast_usize_counter() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("chunk_manager.rs"),
        tip.join("voxel/chunk_manager.rs"),
        tip.join("systems.rs"),
        tip.join("ecs/systems.rs"),
        tip.join("lod_config.rs"),
        tip.join("lod/lod_config.rs"),
        game.join("gen/voxel/chunk_manager.rs"),
        game.join("gen/ecs/systems.rs"),
        game.join("gen/lod/lod_config.rs"),
        game.join("gen/scenes/scene_file.rs"),
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
            line.contains("(i as usize)") && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-361: game-core/tip product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-361 RED: tip/product (i as usize) in:\n  {}",
        bad_paths.join("\n  ")
    );
}
