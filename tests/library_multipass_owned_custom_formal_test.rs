//! Library multipass: WJ bare `Custom` formals that forward into owned-consuming
//! callees must keep owned Rust formals (not demote to `&T` / `&mut T`).
//!
//! Take/restore bodies still emit `&mut` (see `bug_mut_param_passthrough_*`).
//! Readonly-only Custom may demote to `&T` (WDB-097).

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

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn library_multipass_owned_custom_wrapper_keeps_owned_formal() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "graph/types.wj",
        r#"
pub struct DenseCsr {
    pub offsets: Vec<int>,
    pub neighbors: Vec<int>,
}
"#,
    );
    test.add_file(
        "graph/engine.wj",
        r#"
use crate::graph::types::DenseCsr

pub fn consume_neighbors(csr: DenseCsr) -> Vec<int> {
    csr.neighbors
}

pub fn run_dense(csr: DenseCsr) -> Vec<int> {
    consume_neighbors(csr)
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("graph/engine.rs")
        .or_else(|| map.get("engine.rs"))
        .expect("engine.rs output");
    assert!(
        rs.contains("fn run_dense(csr: DenseCsr")
            && rs.contains("consume_neighbors(csr)")
            && !rs.contains("fn run_dense(csr: &DenseCsr")
            && !rs.contains("fn run_dense(csr: &mut DenseCsr")
            && !rs.contains("consume_neighbors(&csr")
            && !rs.contains("consume_neighbors(&mut csr"),
        "owned Custom wrapper into consuming callee must keep owned formal. Got:\n{rs}"
    );
}

#[test]
fn library_multipass_owned_custom_forward_with_field_reads_keeps_owned_formal() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "graph/types.wj",
        r#"
pub struct DenseCsr {
    pub offsets: Vec<int>,
    pub vertex_ids: Vec<int>,
}

pub struct LccEngine {
    pub total: int,
}
"#,
    );
    test.add_file(
        "graph/lcc.wj",
        r#"
use crate::graph::types::DenseCsr
use crate::graph::types::LccEngine

pub fn degree_order(csr: DenseCsr) -> Vec<int> {
    csr.vertex_ids
}

pub fn forward_adj(csr: DenseCsr, order: Vec<int>) -> int {
    csr.offsets.len() as int + order.len() as int
}

pub fn run_dense(csr: DenseCsr) -> LccEngine {
    let n = csr.vertex_ids.len()
    if n == 0 {
        return LccEngine { total: 0 }
    }
    let order = degree_order(csr)
    let _ = forward_adj(csr, order)
    LccEngine { total: n as int }
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("graph/lcc.rs")
        .or_else(|| map.get("lcc.rs"))
        .expect("lcc.rs output");
    assert!(
        rs.contains("fn run_dense(csr: DenseCsr")
            && !rs.contains("fn run_dense(csr: &DenseCsr")
            && !rs.contains("fn run_dense(csr: &mut DenseCsr")
            && rs.contains("degree_order(csr")
            && !rs.contains("degree_order(&csr"),
        "owned Custom with field reads + owned forwards must keep owned formal \
         and owned call into degree_order. Got:\n{rs}"
    );
}
