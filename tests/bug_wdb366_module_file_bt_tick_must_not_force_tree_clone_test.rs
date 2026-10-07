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
//!
//! Isolate must use a **non-Copy** tree (Vec nodes) + string field let
//! (`let name = tree.nodes[i].name`) + mutual recursion through a composite helper.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Node {
    pub id: i32,
    pub name: string,
    pub kids: Vec<i32>,
    pub kind: i32,
}

pub struct Tree {
    pub nodes: Vec<Node>,
    pub root: i32,
}

pub enum Status {
    Ok,
    Fail,
    Run,
}

fn leaf_read(tree: Tree, idx: i32) -> i32 {
    tree.nodes[idx as usize].id
}

pub fn tick_action(tree: Tree, name: string) -> Status {
    if tree.nodes.len() > 0 {
        Status::Ok
    } else {
        Status::Fail
    }
}

pub fn tick_decorator(tree: Tree, parent_idx: i32, child_count: usize, active: Vec<i32>) -> Status {
    if child_count == 0 {
        return Status::Fail
    }
    let cid = leaf_read(tree, parent_idx)
    let s = tick_node(tree, cid, active)
    match s {
        Status::Ok => Status::Fail,
        Status::Fail => Status::Ok,
        Status::Run => Status::Run,
    }
}

pub fn tick_seq(tree: Tree, parent_idx: i32, child_count: usize, active: Vec<i32>) -> Status {
    let mut i = 0
    while i < child_count as i32 {
        let cid = leaf_read(tree, parent_idx)
        let s = tick_node(tree, cid, active)
        match s {
            Status::Fail => return Status::Fail,
            Status::Run => return Status::Run,
            Status::Ok => {},
        }
        i = i + 1
    }
    Status::Ok
}

pub fn tick_node(tree: Tree, node_id: i32, active: Vec<i32>) -> Status {
    let idx = leaf_read(tree, node_id)
    if idx < 0 {
        return Status::Fail
    }
    let child_count = tree.nodes[idx as usize].kids.len()
    let name = tree.nodes[idx as usize].name
    let kind = tree.nodes[idx as usize].kind
    let status = match kind {
        0 => tick_action(tree, name),
        1 => tick_decorator(tree, idx, child_count, active),
        _ => tick_seq(tree, idx, child_count, active),
    }
    active.push(node_id)
    status
}

pub fn tick(tree: Tree) -> Status {
    let mut active: Vec<i32> = Vec::new()
    tick_node(tree, tree.root, active)
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
    assert!(
        rs.contains("fn tick_node(tree: &Tree")
            || rs.contains("fn tick_node(tree: & Tree"),
        "WDB-366 RED: tick_node must demote to &Tree:\n{rs}"
    );
    assert!(
        rs.contains("fn tick_seq(tree: &Tree") || rs.contains("fn tick_seq(tree: & Tree"),
        "WDB-366 RED: tick_seq must demote to &Tree:\n{rs}"
    );
    assert!(
        rs.contains("fn tick_action(tree: &Tree")
            || rs.contains("fn tick_action(tree: & Tree"),
        "WDB-366 RED: tick_action must demote to &Tree:\n{rs}"
    );
    assert!(
        rs.contains("fn tick_decorator(tree: &Tree")
            || rs.contains("fn tick_decorator(tree: & Tree"),
        "WDB-366 RED: tick_decorator must demote to &Tree:\n{rs}"
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
