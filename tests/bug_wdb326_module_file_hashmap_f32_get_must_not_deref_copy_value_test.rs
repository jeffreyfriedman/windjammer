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

//! WDB-326: HashMap get of Copy f32 must not emit `Some(v) => *v` (after `.copied()`).
//!
//! Product tip game-core `ai/astar_grid.rs` / `ai/navmesh.rs`:
//!   `match g_score.get(...).copied() { Some(v) => *v, ... }` → E0614
//!   (`f32` cannot be dereferenced). Source WJ uses `Some(v) => v`.
//! Twin of WDB-134 (i64 get) — Copy float path must bind by value.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
use std::collections::HashMap

pub fn lookup(scores: HashMap<(i32, i32), f32>, x: i32, y: i32) -> f32 {
    let current_g = match scores.get((x, y)) {
        Some(v) => v,
        None => 999999.0,
    }
    current_g
}
"#;

#[test]
fn wdb326_module_file_hashmap_f32_get_must_not_deref_copy_value() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-326 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-326 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("Some(v) => *v") || rs.contains("Some(v)=> *v");
    assert!(
        !bad,
        "WDB-326 RED: Copy f32 HashMap get emitted *v deref:\n{rs}"
    );
    test.cargo_check().expect("WDB-326 cargo-check");
}

#[test]
fn wdb326_tip_out_game_core_astar_navmesh_must_not_deref_copy_f32() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("astar_grid.rs"),
        tip.join("ai/astar_grid.rs"),
        tip.join("navmesh.rs"),
        tip.join("ai/navmesh.rs"),
        game.join("gen/ai/astar_grid.rs"),
        game.join("gen/ai/navmesh.rs"),
        game.join("ai/astar_grid.rs"),
        game.join("ai/navmesh.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("astar/navmesh");
        let bad = text.lines().any(|line| {
            line.contains("Some(v) => *v") || line.contains("Some(v)=> *v")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-326: game-core/tip astar_grid/navmesh missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-326 RED: tip/product still deref Copy f32 via *v in:\n  {}",
        bad_paths.join("\n  ")
    );
}
