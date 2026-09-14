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

//! WDB-217: owned `csr.clone()` into demoted `&mut DenseCsr` must reborrow.
//!
//! Product residual (~8×), tip-out/gen pagerank:
//!   `graph_pagerank_run_dense_pull_fused_gated(csr: &mut DenseCsr, …)`
//!   call `…_gated(csr.clone(), …)` → expected `&mut DenseCsr`, found `DenseCsr`.
//! Prefer pass `csr` (already `&mut`) or owned formal + move.

use std::path::PathBuf;

#[test]
fn wdb217_tip_out_pagerank_must_not_pass_owned_clone_into_mut_ref_csr() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_pagerank_engine.rs"),
        gen.join("graph/graph_pagerank_engine.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("pagerank");
        let demoted = text.contains(
            "fn graph_pagerank_run_dense_pull_fused_gated(csr: &mut DenseCsr",
        );
        let bad = demoted
            && text.contains("graph_pagerank_run_dense_pull_fused_gated(csr.clone(),");
        eprintln!("WDB-217 demoted={} bad={} path={}", demoted, bad, path.display());
        assert!(
            !bad,
            "WDB-217 RED: tip-out/product passes csr.clone() into &mut DenseCsr formal. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-217: pagerank missing");
}
