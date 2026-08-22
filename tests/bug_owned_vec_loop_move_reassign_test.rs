//! Owned Vec formals in a loop with move-then-reassign must pass by value, not `&`.
//!
//! Dogfood: `graph_csr_sort_vertex_neighbors(&neighbors, &weights, ...)` when callee
//! expects `Vec<i64>` / `Vec<f64>` (E0308 in wdb-layers graph_csr_engine).

#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn owned_vec_loop_move_reassign_must_not_prefix_amp() {
    let source = r#"
pub fn sort_pair(a: Vec<int>, b: Vec<int>) -> (Vec<int>, Vec<int>) {
    (a, b)
}

pub fn build_sorted() -> Vec<int> {
    let mut neighbors = Vec::new()
    let mut weights = Vec::new()
    neighbors.push(1)
    weights.push(2)
    let mut vi = 0
    while vi < 1 {
        let sorted = sort_pair(neighbors, weights)
        neighbors = sorted.0
        weights = sorted.1
        vi = vi + 1
    }
    neighbors
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("sort_pair(neighbors, weights)")
            && !generated.contains("sort_pair(&neighbors")
            && !generated.contains("sort_pair(&weights"),
        "owned Vec formals with move-then-reassign must pass by value. Got:\n{generated}"
    );
}

#[test]
fn owned_host_last_use_must_not_prefix_amp() {
    let source = r#"
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
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("consume_host(host2)")
            && !generated.contains("consume_host(&host2)"),
        "owned struct formal on last-use move must pass by value. Got:\n{generated}"
    );
}
