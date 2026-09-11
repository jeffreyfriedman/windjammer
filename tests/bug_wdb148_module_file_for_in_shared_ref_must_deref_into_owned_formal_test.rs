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

//! WDB-148: `for x in &collection` yields `&T`; tip must auto-deref (or `*x`) when
//! passing into an owned `T` / `i64` formal.
//!
//! WindjammerDB CQ-C5 LSQB restore:
//!   `for country in &graph.countries { lsqb_in_neighbors(..., country) }`
//! with `vertex: i64` → E0308 (`expected i64, found &i64`).
//! Same for `for p1 in &graph.persons { lsqb_knows_neighbors(..., p1) }`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod g
pub mod q
"#;

const G: &str = r#"
pub struct Graph {
    pub persons: Vec<i64>,
}

pub fn knows_neighbors(graph: Graph, person: i64) -> u32 {
    person as u32 + graph.persons.len() as u32
}
"#;

const Q: &str = r#"
use crate::g::Graph
use crate::g::knows_neighbors

pub fn count(graph: Graph) -> u32 {
    let mut n: u32 = 0
    for p1 in graph.persons {
        n = n + knows_neighbors(graph, p1)
    }
    n
}

pub fn cap() -> u32 {
    count(Graph { persons: vec![1, 2, 3] })
}
"#;

fn wdb148_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("g.wj", G);
    test.add_file("q.wj", Q);
    test
}

#[test]
fn wdb148_module_file_for_in_shared_ref_must_deref_into_owned_formal() {
    let test = wdb148_fixture();
    let map = test
        .compile()
        .expect("WDB-148 multipass compile should succeed (codegen may still be wrong)");
    let q_rs = map.get("q.rs").expect("q.rs");

    // Product pattern uses `for p1 in &graph.persons` after demotion of graph to &Graph.
    // If tip emits `for p1 in &graph.persons` + `knows_neighbors(..., p1)` without * → RED.
    let shared_for = q_rs.contains("for p1 in &") || q_rs.contains("for p1 in graph.persons");
    let missing_deref = q_rs.contains("knows_neighbors(")
        && q_rs.contains(", p1)")
        && !q_rs.contains(", *p1)")
        && q_rs.contains("for p1 in &");

    eprintln!("WDB-148 q.rs:\n{q_rs}");
    eprintln!("shared_for={shared_for} missing_deref={missing_deref}");

    if missing_deref {
        panic!(
            "WDB-148 RED: for-in shared ref binding passed to owned i64 formal without deref. \
             Product: lsqb_query6/9 knows_neighbors(graph, p1)."
        );
    }

    test.cargo_check().expect(
        "WDB-148: for-in ref → owned formal must cargo-check.",
    );
}
