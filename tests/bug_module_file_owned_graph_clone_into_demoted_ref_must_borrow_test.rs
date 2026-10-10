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

//! Regression: module file owned graph clone into demoted ref must borrow.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod typed
pub mod engine
"#;

const TYPED: &str = r#"
pub struct TypedGraph {
    pub n: int,
}

/// Read-only query — codegen demotes to `&TypedGraph`.
pub fn query_a(graph: TypedGraph) -> int {
    graph.n
}

pub fn query_b(graph: TypedGraph) -> int {
    graph.n + 1
}
"#;

const ENGINE: &str = r#"
use crate::typed::TypedGraph
use crate::typed::query_a
use crate::typed::query_b

pub fn run_on_graph(graph: TypedGraph, query_id: int) -> int {
    if query_id == 4 {
        // query_a(graph.clone()) into demoted & — must &graph
        return query_a(graph.clone())
    }
    if query_id == 5 {
        return query_b(graph.clone())
    }
    0
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("typed.wj", TYPED);
    test.add_file("engine.wj", ENGINE);
    test
}

#[test]
fn module_file_owned_graph_clone_into_demoted_ref_must_borrow() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let typed_rs = map.get("typed.rs").expect("typed.rs");
    let engine_rs = map.get("engine.rs").expect("engine.rs");

    eprintln!("typed.rs:\n{typed_rs}\nengine.rs:\n{engine_rs}");

    let demoted = {
        let i = typed_rs.find("fn query_a").unwrap_or(0);
        let sl = &typed_rs[i..typed_rs.len().min(i + 80)];
        sl.contains("graph: &TypedGraph") || sl.contains("graph:&TypedGraph")
    };
    let bad = engine_rs.contains("query_a(graph.clone())")
        && !engine_rs.contains("query_a(&graph");
    let good = engine_rs.contains("query_a(&graph")
        || engine_rs.contains("query_a(&graph.clone()");

    if demoted && bad && !good {
        panic!(
            "RED: demoted &TypedGraph received graph.clone(). \
             Got:\n{engine_rs}"
        );
    }
    if demoted {
        assert!(
            good || !bad,
            "demoted &Graph must borrow. Got:\n{engine_rs}"
        );
    }
}
