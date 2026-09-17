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

//! WDB-258: demoted `&Vec` into owned materialize dsts/weights must clone.
//!
//! Twin of WDB-241 (ecs `&ids`→owned Vec). Tip rematerialized
//! `graph_materialize_from_edge_lists(srcs: Vec, dsts: Vec, weights: Vec, …)`
//! but tip-out/gen still emits:
//!   `graph_materialize_from_edge_lists(self.srcs.clone(), &self.dsts, &self.weights, …)`
//!   `graph_materialize_from_edge_lists(lists.0, &lists.1, &lists.2, …)`
//! → E0308 expected `Vec`, found `&Vec`.
//! Signature-driven: clone/`to_vec` for owned formals (or rematerialize demote).
//! WDB-239 covers the opposite demoted-formal case on sql_query_port.

use std::path::PathBuf;

#[test]
fn wdb258_tip_out_must_clone_ref_vec_into_owned_materialize_dsts_weights() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let mat_paths = [
        tip.join("graph_materialize_port.rs"),
        gen.join("graph/graph_materialize_port.rs"),
    ];
    let mut owned_all = false;
    for path in &mat_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("materialize");
        if text.contains(
            "fn graph_materialize_from_edge_lists(srcs: Vec<i64>, dsts: Vec<i64>, weights: Vec<f64>",
        ) {
            owned_all = true;
            break;
        }
    }
    assert!(
        owned_all,
        "WDB-258: owned-all materialize edge-list formals missing"
    );

    let call_paths = [
        tip.join("graph_csr_integrate.rs"),
        tip.join("graph_write_port.rs"),
        gen.join("graph/graph_csr_integrate.rs"),
        gen.join("graph/graph_write_port.rs"),
    ];
    let mut saw = false;
    let mut any_bad = false;
    let mut bad_path = String::new();
    for path in &call_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("caller");
        let bad = text.contains("graph_materialize_from_edge_lists(self.srcs.clone(), &self.dsts, &self.weights,")
            || text.contains("graph_materialize_from_edge_lists(lists.0, &lists.1, &lists.2,");
        eprintln!("WDB-258 bad={} path={}", bad, path.display());
        if bad {
            any_bad = true;
            bad_path = path.display().to_string();
        }
    }
    assert!(saw, "WDB-258: csr_integrate/write_port missing");
    assert!(
        !any_bad,
        "WDB-258 RED: tip-out/product passes &Vec into owned materialize dsts/weights. {}",
        bad_path
    );
}
