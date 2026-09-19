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

//! WDB-233: `self.csr.clone()` into demoted `&mut DenseCsr` must reborrow.
//!
//! Tip keeps many graph_*_run_dense* formals owned (`DenseCsr`) or `&mut` —
//! clone into owned is correct. Only flag clone into `&mut DenseCsr` callees.

use std::path::PathBuf;

#[test]
fn wdb233_tip_out_analytics_must_not_pass_owned_csr_clone_into_mut_ref() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
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
        let text = std::fs::read_to_string(path).expect("session");
        // Tip emits `&mut self.csr` into &mut formals; owned formals use `.clone()`.
        // RED only when clone is passed where tip still has &mut formals for that name.
        // Heuristic without reading every formal file: clone into pagerank/wcc/cdlp
        // run_dense (known &mut) is bad; bfs/sssp/lcc/multi_source may be owned.
        let bad = text.contains("graph_pagerank_run_dense(self.csr.clone(),")
            || text.contains("graph_wcc_run_dense(self.csr.clone()")
            || text.contains("graph_cdlp_run_dense(self.csr.clone(),")
            || text.contains("graph_bfs_run_dense_batch(self.csr.clone(),")
            || text.contains("graph_sssp_run_dense_batch(self.csr.clone(),");
        eprintln!("WDB-233 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-233 RED: tip-out/product passes owned csr.clone() into &mut DenseCsr. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-233: graph_analytics_session missing");
}
