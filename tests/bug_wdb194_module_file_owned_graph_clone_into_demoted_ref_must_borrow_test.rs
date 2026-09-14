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

//! WDB-194: owned `graph.clone()` into demoted `&LsqbTypedGraph` must borrow.
//!
//! Product residual (~4× &LsqbTypedGraph←LsqbTypedGraph), gen lsqb_query_engine:
//!   `lsqb_query4(graph: &LsqbTypedGraph)` called as `lsqb_query4(graph.clone())`
//! → E0308. Mixed: some queries correctly use `&graph.clone()`, others pass owned
//! `graph.clone()`. Signature-driven — demoted & formal must get `&graph`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod typed
pub mod engine
"#;

const TYPED: &str = r#"
pub struct TypedGraph {
    pub n: int,
}

/// Read-only query — tip demotes to `&TypedGraph` (product lsqb_query4/5/7/8).
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
        // Product: query_a(graph.clone()) into demoted & — must &graph
        return query_a(graph.clone())
    }
    if query_id == 5 {
        return query_b(graph.clone())
    }
    0
}
"#;

fn wdb194_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("typed.wj", TYPED);
    test.add_file("engine.wj", ENGINE);
    test
}

#[test]
fn wdb194_module_file_owned_graph_clone_into_demoted_ref_must_borrow() {
    let test = wdb194_fixture();
    let map = test
        .compile()
        .expect("WDB-194 multipass compile should succeed");
    let typed_rs = map.get("typed.rs").expect("typed.rs");
    let engine_rs = map.get("engine.rs").expect("engine.rs");

    eprintln!("WDB-194 typed.rs:\n{typed_rs}\nengine.rs:\n{engine_rs}");

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
            "WDB-194 RED: demoted &TypedGraph received graph.clone(). \
             Product: lsqb_query4(graph.clone()). Got:\n{engine_rs}"
        );
    }
    if demoted {
        assert!(
            good || !bad,
            "WDB-194: demoted &Graph must borrow. Got:\n{engine_rs}"
        );
    }
}

#[test]
fn wdb194_product_lsqb_must_borrow_graph_into_demoted_query() {
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let engine = gen.join("graph/lsqb_query_engine.rs");
    let typed = gen.join("graph/lsqb_typed_graph.rs");
    if !engine.exists() {
        eprintln!("WDB-194: skip — lsqb engine missing");
        return;
    }
    let engine_text = std::fs::read_to_string(&engine).expect("engine");
    let typed_text = if typed.exists() {
        std::fs::read_to_string(&typed).unwrap_or_default()
    } else {
        String::new()
    };
    let demoted = engine_text.contains("fn lsqb_query4(graph: &LsqbTypedGraph")
        || engine_text.contains("fn lsqb_query7(graph: &LsqbTypedGraph");
    // Only demoted formals (query4/7) — query5/8 stay owned and may take graph.clone().
    let bad = engine_text.contains("lsqb_query4(graph.clone())")
        || engine_text.contains("lsqb_query7(graph.clone())");
    eprintln!(
        "WDB-194 product demoted={} bad={} path={}",
        demoted,
        bad,
        engine.display()
    );
    assert!(
        !(demoted && bad),
        "WDB-194 RED: product still passes owned graph.clone() into demoted lsqb_query4/7. {}",
        engine.display()
    );
}
