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
    feature = "codegen_tests",
))]

//! WDB-269: owned `graph.clone()` into demoted `&LsqbTypedGraph` neighbors must reborrow.
//!
//! Twin / inverse of WDB-220 (when formals were owned). Tip demotes
//!   `lsqb_in_neighbors(graph: &LsqbTypedGraph, …)` / out / knows
//! but tip-out/gen still emits `lsqb_*_neighbors(graph.clone(), …)` from owned
//! `graph: LsqbTypedGraph` locals → E0308 expected `&LsqbTypedGraph`, found owned.
//! Signature-driven: pass `&graph` / bare `graph` when formal is `&LsqbTypedGraph`.

use std::path::PathBuf;

#[test]
fn wdb269_tip_out_lsqb_must_reborrow_owned_graph_clone_into_demoted_neighbors() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");

    let pairs = [
        (tip.join("lsqb_typed_graph.rs"), tip.join("lsqb_query_engine.rs")),
        (
            gen.join("graph/lsqb_typed_graph.rs"),
            gen.join("graph/lsqb_query_engine.rs"),
        ),
    ];
    let mut saw = false;
    let mut any_bad = false;
    let mut bad_path = String::new();
    for (formal_path, call_path) in &pairs {
        if !formal_path.exists() || !call_path.exists() {
            continue;
        }
        saw = true;
        let formal = std::fs::read_to_string(formal_path).expect("typed_graph");
        let demoted = formal.contains("fn lsqb_in_neighbors(graph: &LsqbTypedGraph")
            || formal.contains("fn lsqb_out_neighbors(graph: &LsqbTypedGraph")
            || formal.contains("fn lsqb_knows_neighbors(graph: &LsqbTypedGraph");
        let call = std::fs::read_to_string(call_path).expect("query_engine");
        let bad = demoted
            && (call.contains("lsqb_in_neighbors(graph.clone(),")
                || call.contains("lsqb_out_neighbors(graph.clone(),")
                || call.contains("lsqb_knows_neighbors(graph.clone(),"));
        eprintln!(
            "WDB-269 demoted={} bad={} path={}",
            demoted,
            bad,
            call_path.display()
        );
        if bad {
            any_bad = true;
            bad_path = call_path.display().to_string();
        }
    }
    assert!(saw, "WDB-269: lsqb typed_graph / query_engine missing");
    assert!(
        !any_bad,
        "WDB-269 RED: tip-out/product passes graph.clone() into demoted &LsqbTypedGraph neighbors. {}",
        bad_path
    );
}
