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

//! WDB-248: owned `self.csr.clone()` into demoted `&DenseCsr` multi_source must reborrow.
//!
//! After tip demotion of `graph_bfs_run_dense_multi_source(csr: &DenseCsr, …)`,
//! tip-out/gen graph_analytics_session still emits:
//!   `graph_bfs_run_dense_multi_source(self.csr.clone(), sources)` → E0308
//!   expected `&DenseCsr`, found `DenseCsr`.
//! Twin of WDB-233 (was `&mut`); signature-driven: `&self.csr` / bare reborrow.

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
    for path in &batch_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("batch");
        if text.contains("fn graph_bfs_run_dense_multi_source(csr: &DenseCsr") {
            demoted = true;
            break;
        }
    }
    assert!(
        demoted,
        "WDB-248: demoted &DenseCsr multi_source formal missing"
    );

    let paths = [
        tip.join("graph_analytics_session.rs"),
        gen.join("graph/graph_analytics_session.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("analytics");
        let bad = text.contains("graph_bfs_run_dense_multi_source(self.csr.clone(),")
            && !text.contains("graph_bfs_run_dense_multi_source(&self.csr,")
            && !text.contains("graph_bfs_run_dense_multi_source(&mut self.csr,");
        eprintln!("WDB-248 demoted={} bad={} path={}", demoted, bad, path.display());
        assert!(
            !bad,
            "WDB-248 RED: tip-out/product passes csr.clone() into demoted &DenseCsr multi_source. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-248: graph_analytics_session missing");
}
