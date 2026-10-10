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

//!
//!   `graph_bfs_run_dense_beamer(csr.clone(), …)` while formal is `&DenseCsr`
//!
//! Expected: borrow at call site (`&graph` / `&graph.clone()`) OR keep owned formal.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod graph
pub mod query
"#;

const GRAPH: &str = r#"
pub struct TypedGraph {
    pub vertex_count: u64,
}

pub fn query4(graph: TypedGraph) -> u64 {
    graph.vertex_count
}
"#;

const QUERY: &str = r#"
use crate::graph::TypedGraph
use crate::graph::query4

/// Reuse `g` across calls — demotes `query4` formal to `&TypedGraph` in multipass.
pub fn all_counts(g: TypedGraph) -> u64 {
    let a = query4(g.clone())
    let b = query4(g.clone())
    a + b
}

pub fn cap() -> u64 {
    all_counts(TypedGraph { vertex_count: 3 })
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("graph.wj", GRAPH);
    test.add_file("query.wj", QUERY);
    test
}

#[test]
fn module_file_demoted_struct_formal_must_borrow_clone_call_sites() {
    let mut test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let graph_rs = map.get("graph.rs").expect("graph.rs");
    let query_rs = map.get("query.rs").expect("query.rs");

    let demoted = graph_rs.contains("graph: &TypedGraph") || graph_rs.contains("graph: & TypedGraph");
    let borrows = query_rs.contains("query4(&g") || query_rs.contains("query4(&g.clone()");
    let bad_owned = query_rs.contains("query4(g.clone()") && !borrows;

    if demoted && bad_owned {
        eprintln!("RED graph.rs:\n{graph_rs}\nquery.rs:\n{query_rs}");
    }

    test.cargo_check().expect(
        "RED: demoted &TypedGraph + owned .clone() call sites must borrow. ",
    );
}
