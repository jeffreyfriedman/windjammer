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
))]

//! Dogfood: `behavior_tree/executor.wj` stack-overflowed during library codegen when the
//! global registry was large (~15k sigs). Minimal shape: mutually recursive free functions
//! forwarding owned `Vec<i32>` + `BehaviorTree` through `match` / `while` (tick_node ↔ composites).
//!
//! Root cause: `param_should_emit_borrowed_delegation_formal` re-entered without a guard when
//! formal emission called `param_keeps_owned_engine_key_facade`, and was recomputed many times
//! per param (deep stack). Fix: per-function memo + reentrancy guard on borrow-delegation queries.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use std::time::{Duration, Instant};

use integration_test_helpers::MultiFileTest;

const COMPILE_BUDGET: Duration = Duration::from_secs(30);

fn bt_executor_minimal_fixture() -> &'static str {
    r#"
pub enum BehaviorStatus {
    Success,
    Failure,
    Running,
}

pub enum NodeType {
    Action,
    Condition,
    Sequence,
    Selector,
}

pub struct NodeId {
    value: i32,
}

impl NodeId {
    pub fn value(self) -> i32 {
        self.value
    }
}

pub struct Node {
    node_type: NodeType,
    name: string,
    children: Vec<NodeId>,
}

pub struct BehaviorTree {
    nodes: Vec<Node>,
    node_ids: Vec<NodeId>,
    root_id: i32,
    has_root: bool,
}

fn tick_node(
    tree: BehaviorTree,
    node_id: i32,
    active: Vec<i32>,
    running: Vec<i32>,
    success: Vec<i32>,
    failure: Vec<i32>,
) -> BehaviorStatus {
    let idx = find_index_by_id(tree, node_id)
    if idx < 0 {
        return BehaviorStatus::Failure
    }
    let node_type = tree.nodes[idx as usize].node_type
    let child_count = tree.nodes[idx as usize].children.len()
    let status = match node_type {
        NodeType::Action => BehaviorStatus::Success,
        NodeType::Condition => BehaviorStatus::Success,
        NodeType::Sequence => tick_composite_sequence(
            tree,
            idx,
            child_count,
            active,
            running,
            success,
            failure,
        ),
        NodeType::Selector => tick_composite_selector(
            tree,
            idx,
            child_count,
            active,
            running,
            success,
            failure,
        ),
    }
    status
}

fn tick_composite_sequence(
    tree: BehaviorTree,
    parent_idx: i32,
    child_count: usize,
    active: Vec<i32>,
    running: Vec<i32>,
    success: Vec<i32>,
    failure: Vec<i32>,
) -> BehaviorStatus {
    let mut i = 0
    while i < child_count as i32 {
        let cid = get_child_id(tree, parent_idx, i)
        let s = tick_node(tree, cid, active, running, success, failure)
        match s {
            BehaviorStatus::Failure => return BehaviorStatus::Failure,
            BehaviorStatus::Running => return BehaviorStatus::Running,
            BehaviorStatus::Success => {},
        }
        i = i + 1
    }
    BehaviorStatus::Success
}

fn tick_composite_selector(
    tree: BehaviorTree,
    parent_idx: i32,
    child_count: usize,
    active: Vec<i32>,
    running: Vec<i32>,
    success: Vec<i32>,
    failure: Vec<i32>,
) -> BehaviorStatus {
    let mut i = 0
    while i < child_count as i32 {
        let cid = get_child_id(tree, parent_idx, i)
        let s = tick_node(tree, cid, active, running, success, failure)
        match s {
            BehaviorStatus::Success => return BehaviorStatus::Success,
            BehaviorStatus::Running => return BehaviorStatus::Running,
            BehaviorStatus::Failure => {},
        }
        i = i + 1
    }
    BehaviorStatus::Failure
}

fn get_child_id(tree: BehaviorTree, parent_idx: i32, child_offset: i32) -> i32 {
    tree.nodes[parent_idx as usize].children[child_offset as usize].value()
}

fn find_index_by_id(tree: BehaviorTree, node_id: i32) -> i32 {
    let mut i = 0
    while i < tree.node_ids.len() {
        if tree.node_ids[i].value() == node_id {
            return i as i32
        }
        i = i + 1
    }
    -1
}
"#
}

#[test]
fn mutual_recursion_free_fns_codegen_must_not_stack_overflow() {
    let mut t = MultiFileTest::new();

    // Stress multipass registry size (product library builds hold 10k+ signatures).
    let mut mod_lines = String::from("pub mod executor\n");
    for i in 0..64 {
        let name = format!("pad_{:02}", i);
        mod_lines.push_str(&format!("pub mod {}\n", name));
        t.add_file(
            &format!("{}.wj", name),
            &format!("pub fn pad_{}_touch() -> i32 {{ {} }}\n", i, i),
        );
    }
    t.add_file("mod.wj", &mod_lines);
    t.add_file("executor.wj", bt_executor_minimal_fixture());

    let start = Instant::now();
    let map = t.compile().unwrap_or_else(|e| {
        panic!(
            "mutual-recursion BT executor fixture must compile without stack overflow/hang: {e}"
        )
    });
    assert!(
        start.elapsed() < COMPILE_BUDGET,
        "compile took {:?} (budget {:?})",
        start.elapsed(),
        COMPILE_BUDGET
    );

    let rs = map
        .get("executor.rs")
        .expect("executor.rs generated");
    assert!(
        rs.contains("fn tick_node(") && rs.contains("fn tick_composite_sequence("),
        "expected mutual-recursion helpers in output:\n{rs}"
    );
    assert!(
        rs.contains("tick_node("),
        "expected tick_node call sites in output:\n{rs}"
    );
}
