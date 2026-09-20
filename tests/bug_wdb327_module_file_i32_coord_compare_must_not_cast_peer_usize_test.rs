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

//! WDB-327: i32 coords must not compare against `goal as usize` peers.
//!
//! Product tip game-core `ai/astar_grid.rs`:
//!   `if current_x == goal_x as usize` with `current_x: i32`, `goal_x: i32`
//!   → E0308 / E0277. Source WJ: `if current_x == goal_x`.
//! Prefer same-width i32 compare (no spurious `as usize`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn at_goal(current_x: i32, current_y: i32, goal_x: i32, goal_y: i32) -> bool {
    current_x == goal_x && current_y == goal_y
}
"#;

/// Product shape: i32 field loaded via `vec[usize_idx].x` then compared to i32 goal.
const SRC_INDEXED_FIELD: &str = r#"
pub struct AStarNode {
    pub x: i32,
    pub y: i32,
}

pub fn at_goal_from_open(open_set: Vec<AStarNode>, best_idx_usize: usize, goal_x: i32, goal_y: i32) -> bool {
    let current_x = open_set[best_idx_usize].x
    let current_y = open_set[best_idx_usize].y
    current_x == goal_x && current_y == goal_y
}
"#;

#[test]
fn wdb327_module_file_i32_coord_compare_must_not_cast_peer_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-327 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-327 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("as usize");
    assert!(
        !bad,
        "WDB-327 RED: i32==i32 compare emitted as usize peer:\n{rs}"
    );
    test.cargo_check().expect("WDB-327 cargo-check");
}

#[test]
fn wdb327_module_file_indexed_i32_field_must_not_cast_goal_to_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC_INDEXED_FIELD);
    let map = test.compile().expect("WDB-327 indexed compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-327 indexed MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("goal_x as usize")
        || rs.contains("goal_y as usize")
        || rs.contains("current_x as i32")
        || rs.contains("current_y as i32");
    assert!(
        !bad,
        "WDB-327 RED: indexed i32 field compare cast goal/current through usize:\n{rs}"
    );
    test.cargo_check().expect("WDB-327 indexed cargo-check");
}

#[test]
fn wdb327_tip_out_game_core_astar_must_not_compare_i32_to_usize_goal() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("astar_grid.rs"),
        tip.join("ai/astar_grid.rs"),
        game.join("gen/ai/astar_grid.rs"),
        game.join("ai/astar_grid.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("astar");
        let bad = text.lines().any(|line| {
            line.contains("goal_x as usize") || line.contains("goal_y as usize")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-327: game-core/tip astar_grid missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-327 RED: tip/product compares i32 to goal as usize in:\n  {}",
        bad_paths.join("\n  ")
    );
}
