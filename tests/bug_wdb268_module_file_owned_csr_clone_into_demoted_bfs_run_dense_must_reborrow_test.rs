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

//! WDB-268: owned `csr.clone()` into demoted `&DenseCsr` `graph_bfs_run_dense` must reborrow.
//!
//! Twin of WDB-248/256. Tip demotes `graph_bfs_run_dense(csr: &DenseCsr, …)` but
//! tip-out/gen incremental still emits:
//!   `graph_bfs_run_dense(csr.clone(), source)` → E0308.
//! Gen may still keep owned `DenseCsr` formal (clone then required).
//! Signature-driven: when formal is `&DenseCsr`, pass `&csr` / bare `csr`.

use std::path::PathBuf;

#[test]
fn wdb268_tip_out_incremental_must_reborrow_owned_csr_clone_into_demoted_bfs_run_dense() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");

    let pairs = [
        (
            tip.join("graph_bfs_engine.rs"),
            tip.join("graph_incremental_views.rs"),
        ),
        (
            gen.join("graph/graph_bfs_engine.rs"),
            gen.join("graph/graph_incremental_views.rs"),
        ),
        (
            tip.join("graph_bfs_engine.rs"),
            tip.join("graph_analytics_session.rs"),
        ),
        (
            gen.join("graph/graph_bfs_engine.rs"),
            gen.join("graph/graph_analytics_session.rs"),
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
        let formal_text = std::fs::read_to_string(formal_path).expect("bfs formal");
        let demoted = formal_text.contains("fn graph_bfs_run_dense(csr: &DenseCsr");
        let call = std::fs::read_to_string(call_path).expect("call site");
        let bad = demoted
            && (call.contains("graph_bfs_run_dense(csr.clone(),")
                || call.contains("graph_bfs_run_dense(self.csr.clone(),"));
        eprintln!(
            "WDB-268 demoted={} bad={} path={}",
            demoted,
            bad,
            call_path.display()
        );
        if bad {
            any_bad = true;
            bad_path = call_path.display().to_string();
        }
    }
    assert!(saw, "WDB-268: bfs / incremental / analytics missing");
    assert!(
        !any_bad,
        "WDB-268 RED: tip-out/product passes csr.clone() into demoted &DenseCsr bfs_run_dense. {}",
        bad_path
    );
}
