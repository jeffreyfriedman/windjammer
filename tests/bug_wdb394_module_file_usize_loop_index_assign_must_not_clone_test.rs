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

//! WDB-394: Copy usize loop index assigned to `best_idx` must not `i.clone()`.
//!
//! Product gen/ai/navmesh.rs:
//!   best_idx = i.clone();
//! WJ source is `best_idx = i` inside `while i < nodes.len()`.
//! Same smell in astar_grid / reverb_zones / meshing (`max_idx = j.clone()`).
//! Distinct from WDB-343 (i32 formals) and WDB-391 (u32 helper return).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Node {
    pub f_score: f32,
}

pub fn find_min(nodes: Vec<Node>) -> i32 {
    if nodes.len() == 0 {
        return -1
    }
    let mut best_idx = 0
    let mut best_f = nodes[0].f_score
    let mut i = 1
    while i < nodes.len() {
        if nodes[i].f_score < best_f {
            best_f = nodes[i].f_score
            best_idx = i
        }
        i = i + 1
    }
    best_idx as i32
}
"#;

#[test]
fn wdb394_module_file_usize_loop_index_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-394 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-394 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("i.clone()") || rs.contains("best_idx = i.clone()");
    assert!(
        !bad,
        "WDB-394 RED: Copy loop index cloned on assign:\n{rs}"
    );
    test.cargo_check().expect("WDB-394 cargo-check");
}

#[test]
fn wdb394_tip_out_game_core_best_idx_must_not_clone_loop_index() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("ai/navmesh.rs"),
        tip.join("navmesh.rs"),
        game.join("gen/ai/navmesh.rs"),
        tip.join("ai/astar_grid.rs"),
        game.join("gen/ai/astar_grid.rs"),
        tip.join("audio/reverb_zones.rs"),
        game.join("gen/audio/reverb_zones.rs"),
        tip.join("voxel/meshing.rs"),
        game.join("gen/voxel/meshing.rs"),
        tip.join("world_partition/streaming.rs"),
        game.join("gen/world_partition/streaming.rs"),
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
            let t = line.trim_start();
            !t.starts_with("//")
                && (t.contains("best_idx = i.clone()")
                    || t.contains("best_zone = i.clone()")
                    || t.contains("furthest_visible = i.clone()")
                    || t.contains("max_idx = j.clone()")
                    || t.contains("found_idx = j.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-394: best_idx product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-394 RED: tip/product loop-index .clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
