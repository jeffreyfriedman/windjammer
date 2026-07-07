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

//! Multipass library compilation must NOT flag ownership collisions when the
//! same function is re-registered with refined ownership (stub → converged).
//!
//! Bug: During multipass library compilation, `update_params(nodes: Vec<Record>, ...)`
//! is first registered with all-Owned params (declaration stub), then refined to
//! [Borrowed, Owned, Owned] after body analysis. The merge() step flags this as
//! an ownership collision, which suppresses auto-borrow at call sites.
//!
//! Result: `update_params(nodes, pid, val)` instead of `update_params(&nodes, pid, val)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn test_multipass_stub_to_converged_does_not_flag_false_collision() {
    // Models the actual bt_serialization.wj pattern: update_params reads nodes
    // (indexing, .len()) but creates a NEW output Vec. The `nodes` param is
    // read-only and should be inferred as Borrowed.
    let mut test = MultiFileTest::new();
    test.add_file(
        "ai/bt_serialization.wj",
        r#"
pub struct BtNodeRecord {
    id: i32,
    params: string,
}

pub fn update_params(nodes: Vec<BtNodeRecord>, node_id: i32, value: string) -> Vec<BtNodeRecord> {
    let mut out: Vec<BtNodeRecord> = Vec::new()
    let mut i = 0
    while i < nodes.len() {
        let mut rec = nodes[i]
        if rec.id == node_id {
            rec.params = value
        }
        out.push(rec)
        i = i + 1
    }
    out
}

pub fn process_all(nodes: Vec<BtNodeRecord>) -> Vec<BtNodeRecord> {
    let mut result = nodes
    result = update_params(result, 1, "test")
    result = update_params(result, 2, "other")
    result
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map
        .get("ai/bt_serialization.rs")
        .expect("bt_serialization.rs");

    // update_params reads nodes (indexing, .len()) and creates a new output Vec.
    // Body analysis should infer Borrowed → param becomes &Vec<BtNodeRecord>.
    // The call site must auto-borrow: update_params(&result, 1, "test")
    assert!(
        rs.contains("update_params(&result,")
            || rs.contains("update_params(& result,")
            || rs.contains("update_params(&mut result,"),
        "multipass refinement must not suppress auto-borrow. \
         Expected `update_params(&result, ...)` but got owned pass. Got:\n{rs}"
    );
}

#[test]
fn test_multipass_cross_file_stub_to_converged_does_not_flag_false_collision() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "ai/bt_helpers.wj",
        r#"
pub struct BtNodeRecord {
    id: i32,
    params: string,
}

pub fn update_params(nodes: Vec<BtNodeRecord>, node_id: i32, value: string) -> Vec<BtNodeRecord> {
    let mut out: Vec<BtNodeRecord> = Vec::new()
    let mut i = 0
    while i < nodes.len() {
        let mut rec = nodes[i]
        if rec.id == node_id {
            rec.params = value
        }
        out.push(rec)
        i = i + 1
    }
    out
}
"#,
    );
    test.add_file(
        "ai/bt_serialization.wj",
        r#"
use crate::ai::bt_helpers::{BtNodeRecord, update_params}

pub fn process_all(nodes: Vec<BtNodeRecord>) -> Vec<BtNodeRecord> {
    let mut result = nodes
    result = update_params(result, 1, "test")
    result = update_params(result, 2, "other")
    result
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map
        .get("ai/bt_serialization.rs")
        .expect("bt_serialization.rs");

    assert!(
        rs.contains("update_params(&result,")
            || rs.contains("update_params(& result,")
            || rs.contains("update_params(&mut result,"),
        "cross-file multipass refinement must not suppress auto-borrow. \
         Expected `update_params(&result, ...)` but got owned pass. Got:\n{rs}"
    );
}
