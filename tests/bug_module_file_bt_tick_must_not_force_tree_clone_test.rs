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

//! Regression: module file bt tick must not force tree clone.
//!
//! Isolate must use a **non-Copy** tree (Vec nodes) + string field let
//! (`let name = tree.nodes[i].name`) + mutual recursion through a composite helper.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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
fn module_file_bt_tick_must_not_force_tree_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("tree.clone()");
    assert!(
        !bad,
        "RED: tick path forced tree.clone():\n{rs}"
    );
    assert!(
        rs.contains("fn tick_node(tree: &Tree")
            || rs.contains("fn tick_node(tree: & Tree"),
        "RED: tick_node must demote to &Tree:\n{rs}"
    );
    assert!(
        rs.contains("fn tick_seq(tree: &Tree") || rs.contains("fn tick_seq(tree: & Tree"),
        "RED: tick_seq must demote to &Tree:\n{rs}"
    );
    assert!(
        rs.contains("fn tick_action(tree: &Tree")
            || rs.contains("fn tick_action(tree: & Tree"),
        "RED: tick_action must demote to &Tree:\n{rs}"
    );
    assert!(
        rs.contains("fn tick_decorator(tree: &Tree")
            || rs.contains("fn tick_decorator(tree: & Tree"),
        "RED: tick_decorator must demote to &Tree:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
