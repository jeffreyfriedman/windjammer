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

//! WDB-081 / WDB-082 multipass codegen regression gates (WindjammerDB CSR + HashMap i64).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::fs;
use std::path::PathBuf;

fn fixture(relative: &str) -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/library_multipass")
            .join(relative),
    )
    .unwrap_or_else(|e| panic!("missing fixture {relative}: {e}"))
}

/// WDB-081: CSR view reused in while-loop helpers must borrow, not clone neighbor arrays.
#[test]
fn test_library_multipass_graph_csr_view_loop_must_borrow_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        "pub mod graph_csr_view_loop;\npub mod graph_csr_view_runner;\n",
    );
    test.add_file("graph_csr_view_loop.wj", &fixture("graph_csr_view_loop.wj"));
    test.add_file("graph_csr_view_runner.wj", &fixture("graph_csr_view_runner.wj"));

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("graph_csr_view_runner.rs")
        .expect("graph_csr_view_runner.rs generated");

    assert!(
        rs.contains("graph_csr_local_out_degree(&view,")
            || rs.contains("graph_csr_local_out_degree(& view,")
            || rs.contains("graph_csr_local_out_degree(view,"),
        "reused CSR view in loop must borrow for cross-module helpers. Got:\n{rs}"
    );
    assert!(
        rs.contains("graph_csr_local_neighbor_at(&view,")
            || rs.contains("graph_csr_local_neighbor_at(& view,")
            || rs.contains("graph_csr_local_neighbor_at(view,"),
        "reused CSR view in inner loop must borrow for neighbor_at. Got:\n{rs}"
    );
    assert!(
        !rs.contains("view.clone()"),
        "must not clone GraphAdjacencyView each loop iteration. Got:\n{rs}"
    );
    assert!(
        !rs.contains("neighbors.clone()") && !rs.contains("offsets.clone()"),
        "must not clone CSR neighbor/offset vecs when passing view into loop helpers. Got:\n{rs}"
    );

    test.assert_compiles_without_error();
}

/// WDB-082: HashMap<i64,_> contains_key in nested triangle loop auto-borrows i64 keys.
#[test]
fn test_library_multipass_hashmap_i64_set_contains_key_in_triangle_loop() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "graph/hashmap_i64_triangle_loop.wj",
        &fixture("hashmap_i64_triangle_loop.wj"),
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("graph/hashmap_i64_triangle_loop.rs")
        .expect("hashmap_i64_triangle_loop.rs generated");

    assert!(
        rs.contains("contains_key(&w)") || rs.contains("contains_key(& w)"),
        "nested triangle loop i64 keys must auto-borrow for HashMap::contains_key. Got:\n{rs}"
    );
    assert!(
        !rs.contains("contains_key(w)") || rs.contains("contains_key(&w"),
        "must not pass owned i64 to contains_key in inner loop. Got:\n{rs}"
    );

    test.assert_compiles_without_error();
}
