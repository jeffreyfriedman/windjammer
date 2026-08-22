//! Library multipass: owned `Vec` formals must not get stale `&local` prefixes.
//!
//! Dogfood: `graph_csr_sort_vertex_neighbors(&neighbors, &weights, ...)` when callee
//! emits `Vec<i64>` / `Vec<f64>` (directory batch / multipass only).

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
fn library_multipass_owned_vec_loop_call_must_not_prefix_amp() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "graph/adjacency.wj",
        r#"
pub struct WeightedGraphEdge {
    pub src_id: int,
    pub dst_id: int,
    pub weight: f64,
}

pub struct GraphAdjacencyView {
    pub neighbors: Vec<int>,
    pub weights: Vec<f64>,
}
"#,
    );
    test.add_file(
        "graph/csr_engine.wj",
        r#"
use crate::graph::adjacency::GraphAdjacencyView
use crate::graph::adjacency::WeightedGraphEdge

pub fn sort_pair(a: Vec<int>, b: Vec<f64>) -> (Vec<int>, Vec<f64>) {
    (a, b)
}

pub fn build_sorted(edges: Vec<WeightedGraphEdge>) -> GraphAdjacencyView {
    let mut neighbors = Vec::new()
    let mut weights = Vec::new()
    let mut ei = 0
    while ei < edges.len() {
        neighbors.push(edges[ei].src_id)
        weights.push(edges[ei].weight)
        ei = ei + 1
    }
    let mut vi = 0
    while vi < 1 {
        let sorted = sort_pair(neighbors, weights)
        neighbors = sorted.0
        weights = sorted.1
        vi = vi + 1
    }
    GraphAdjacencyView { neighbors: neighbors, weights: weights }
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("graph/csr_engine.rs")
        .or_else(|| map.get("csr_engine.rs"))
        .expect("csr_engine.rs output");
    assert!(
        rs.contains("sort_pair(neighbors, weights)")
            && !rs.contains("sort_pair(&neighbors")
            && !rs.contains("sort_pair(&weights"),
        "owned Vec formals in library multipass must pass by value. Got:\n{rs}"
    );
}

#[test]
fn library_multipass_owned_host_last_use_must_not_prefix_amp() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "graph/host.wj",
        r#"
pub struct Host {
    pub count: int,
}

pub fn consume_host(host: Host) -> int {
    host.count
}

pub fn caller(host: Host) -> int {
    let mut host2 = host
    host2.count = host2.count + 1
    consume_host(host2)
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("graph/host.rs")
        .or_else(|| map.get("host.rs"))
        .expect("host.rs output");
    assert!(
        rs.contains("consume_host(host2)") && !rs.contains("consume_host(&host2)"),
        "owned struct formal on last-use move must pass by value in multipass. Got:\n{rs}"
    );
}
