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

//! WDB-220: demoted `&mut LsqbTypedGraph` into owned `LsqbTypedGraph` must clone.
//!
//! Product residual (~37×), tip-out/gen lsqb_query_engine:
//!   `lsqb_out_neighbors(graph: LsqbTypedGraph, …)` called with bare `graph`
//!   while caller formal is `graph: &mut LsqbTypedGraph`
//! → expected `LsqbTypedGraph`, found `&mut LsqbTypedGraph`.
//! Twin of WDB-175/219. Signature-driven.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str =
    include_str!("fixtures/library_multipass/wdb220_mut_ref_graph_into_owned_must_clone.wj");

#[test]
fn wdb220_codegen_demoted_mut_ref_into_owned_graph_must_clone() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let callee_owned = rs.contains("fn out_neighbors(graph: LsqbTypedGraph")
        || rs.contains("out_neighbors(graph: LsqbTypedGraph");
    let caller_demoted = rs.contains("fn query(graph: &mut LsqbTypedGraph")
        || rs.contains("query(graph: &mut LsqbTypedGraph");
    let bare = rs.contains("out_neighbors(graph,") && !rs.contains("out_neighbors(graph.clone(),");
    let bad = callee_owned && caller_demoted && bare;
    eprintln!(
        "WDB-220 codegen owned={} demoted={} bare={} bad={}\n{rs}",
        callee_owned, caller_demoted, bare, bad
    );
    assert!(
        !bad,
        "WDB-220: demoted &mut graph into owned out_neighbors must clone. Generated:\n{rs}"
    );
    assert!(ok, "WDB-220 fixture must cargo-check. Generated:\n{rs}");
}

use std::path::PathBuf;

#[test]
fn wdb220_tip_out_lsqb_must_clone_mut_ref_graph_into_owned_neighbors() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let engine_paths = [
        tip.join("lsqb_query_engine.rs"),
        gen.join("graph/lsqb_query_engine.rs"),
    ];
    let typed_paths = [
        tip.join("lsqb_typed_graph.rs"),
        gen.join("graph/lsqb_typed_graph.rs"),
    ];
    let mut typed = String::new();
    for path in &typed_paths {
        if path.exists() {
            typed = std::fs::read_to_string(path).expect("typed");
            break;
        }
    }
    assert!(!typed.is_empty(), "WDB-220: lsqb_typed_graph missing");
    let owned_neighbors = typed.contains("fn lsqb_out_neighbors(graph: LsqbTypedGraph")
        || typed.contains("fn lsqb_in_neighbors(graph: LsqbTypedGraph");

    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("engine");
        let demoted = text.contains("graph: &mut LsqbTypedGraph");
        // Any bare `graph,` forward is RED even if other sites correctly `.clone()`.
        let bare = text.contains("lsqb_out_neighbors(graph,")
            || text.contains("lsqb_in_neighbors(graph,");
        let bad = owned_neighbors && demoted && bare;
        eprintln!(
            "WDB-220 owned_neighbors={} demoted={} bare={} bad={} path={}",
            owned_neighbors,
            demoted,
            bare,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-220 RED: tip-out/product passes &mut graph into owned neighbors without clone. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-220: lsqb_query_engine missing");
}
