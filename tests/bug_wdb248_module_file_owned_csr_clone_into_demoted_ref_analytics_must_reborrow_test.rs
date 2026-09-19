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

//! WDB-248: `self.csr` into `graph_bfs_run_dense_multi_source` must match formal ownership.
//!
//! Tip historically demoted `csr: &DenseCsr` while tip-out still emitted
//! `self.csr.clone()` → E0308. Tip now keeps owned `csr: DenseCsr` + `self.csr.clone()`
//! (signature-driven). Gate accepts either demoted+reborrow or owned+clone/move.

use std::path::PathBuf;

#[test]
fn wdb248_tip_out_analytics_must_reborrow_csr_clone_into_demoted_ref_multi_source() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let batch_paths = [
        tip.join("graph_batch_engine.rs"),
        gen.join("graph/graph_batch_engine.rs"),
    ];
    let mut demoted = false;
    let mut owned = false;
    for path in &batch_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("batch");
        if text.contains("fn graph_bfs_run_dense_multi_source(csr: &DenseCsr") {
            demoted = true;
            break;
        }
        if text.contains("fn graph_bfs_run_dense_multi_source(csr: DenseCsr") {
            owned = true;
            break;
        }
    }
    assert!(
        demoted || owned,
        "WDB-248: multi_source DenseCsr formal missing (owned or demoted &)"
    );

    // Prefer tip-out when present (gen may lag behind tip multipass).
    let paths = if tip.join("graph_analytics_session.rs").exists() {
        vec![tip.join("graph_analytics_session.rs")]
    } else {
        vec![gen.join("graph/graph_analytics_session.rs")]
    };
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("analytics");
        let bad = if demoted {
            text.contains("graph_bfs_run_dense_multi_source(self.csr.clone(),")
                && !text.contains("graph_bfs_run_dense_multi_source(&self.csr,")
                && !text.contains("graph_bfs_run_dense_multi_source(&mut self.csr,")
        } else {
            // Owned formal: &DenseCsr into DenseCsr is the RED shape.
            text.contains("graph_bfs_run_dense_multi_source(&self.csr,")
                && !text.contains("graph_bfs_run_dense_multi_source(self.csr.clone(),")
                && !text.contains("graph_bfs_run_dense_multi_source(self.csr,")
        };
        eprintln!(
            "WDB-248 demoted={} owned={} bad={} path={}",
            demoted,
            owned,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-248 RED: tip-out/product call-site ownership mismatches multi_source formal. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-248: graph_analytics_session missing");
}
