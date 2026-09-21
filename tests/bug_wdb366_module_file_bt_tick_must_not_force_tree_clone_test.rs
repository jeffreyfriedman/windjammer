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

//! WDB-366: BT `tick_node` must not force `tree.clone()` via owned `BehaviorTree` formal.
//!
//! Product tip game-core `behavior_tree/executor.rs`:
//!   `tick_node(tree.clone(), cid, …)` / `tick_decorator_*(tree.clone(), …)`
//!   with `tick_node(tree: BehaviorTree, …)` owned — read-only walk should borrow.
//! Prefer `tree: &BehaviorTree` + `tick_node(tree, …)`. Twin of WDB-360 (encode owned).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Tree {
    pub root: i32,
}

pub fn tick_node(tree: Tree, node_id: i32) -> i32 {
    tree.root + node_id
}

pub fn tick_children(tree: Tree, a: i32, b: i32) -> i32 {
    let x = tick_node(tree, a)
    let y = tick_node(tree, b)
    x + y
}
"#;

#[test]
fn wdb366_module_file_bt_tick_must_not_force_tree_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-366 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-366 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("tree.clone()");
    assert!(
        !bad,
        "WDB-366 RED: tick path forced tree.clone():\n{rs}"
    );
    test.cargo_check().expect("WDB-366 cargo-check");
}

#[test]
fn wdb366_tip_out_game_core_bt_must_not_tree_clone() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("executor.rs"),
        tip.join("behavior_tree/executor.rs"),
        game.join("gen/behavior_tree/executor.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("executor");
        let bad = text.lines().any(|line| {
            line.contains("tree.clone()") && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-366: game-core/tip BT executor missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-366 RED: tip/product tree.clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
