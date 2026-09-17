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

//! WDB-239: owned `Vec` into demoted `&Vec` materialize formals must borrow.
//!
//! Product residual tip-out/gen graph_sql_query_port (~4× &Vec←Vec):
//!   `graph_materialize_from_edge_lists(srcs: Vec<i64>, dsts: &Vec<i64>, weights: &Vec<f64>, …)`
//!   called as `graph_materialize_from_edge_lists(srcs, dsts, weights, false)` → E0308.
//! Signature-driven: pass `&dsts`, `&weights`. Twin of WDB-205/238 (owned into demoted Vec).
//!
//! Tip truth (2026-09-17): multipass keeps all three Vec formals Owned — call sites move.
//! Gate accepts owned formals + owned call, or demoted formals + borrowed call.

use std::path::PathBuf;

#[test]
fn wdb239_tip_out_sql_must_borrow_owned_vecs_into_demoted_materialize() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let mat_paths = [
        tip.join("graph_materialize_port.rs"),
        gen.join("graph/graph_materialize_port.rs"),
    ];
    let mut demoted = false;
    let mut owned_all = false;
    for path in &mat_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("materialize");
        if text.contains("fn graph_materialize_from_edge_lists(srcs: Vec<i64>, dsts: &Vec<i64>")
            || text.contains("dsts: &Vec<i64>, weights: &Vec<f64>")
        {
            demoted = true;
            break;
        }
        if text.contains(
            "fn graph_materialize_from_edge_lists(srcs: Vec<i64>, dsts: Vec<i64>, weights: Vec<f64>",
        ) {
            owned_all = true;
        }
    }
    assert!(
        demoted || owned_all,
        "WDB-239: materialize edge-list formals missing (demoted or owned)"
    );

    let paths = [
        tip.join("graph_sql_query_port.rs"),
        gen.join("graph/graph_sql_query_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("sql");
        let bad = demoted
            && text.contains("graph_materialize_from_edge_lists(srcs, dsts, weights,")
            && !text.contains("graph_materialize_from_edge_lists(srcs, &dsts, &weights,")
            && !text.contains("graph_materialize_from_edge_lists(srcs, &dsts, weights,");
        eprintln!(
            "WDB-239 demoted={} owned_all={} bad={} path={}",
            demoted,
            owned_all,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-239 RED: tip-out/product passes owned dsts/weights into demoted &Vec materialize. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-239: graph_sql_query_port missing");
}
