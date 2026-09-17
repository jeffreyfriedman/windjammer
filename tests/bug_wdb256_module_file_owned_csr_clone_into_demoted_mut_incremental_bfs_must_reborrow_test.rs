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

//! WDB-256: owned `csr.clone()` into demoted `&mut DenseCsr` BFS (incremental) must reborrow.
//!
//! Twin of WDB-233 (analytics `self.csr.clone()`). Product residual tip-out/gen
//! graph_incremental_views:
//!   `graph_bfs_run_dense(csr.clone(), source)`
//! while formal is `&mut DenseCsr` → E0308.
//! Signature-driven: pass `&mut csr`.

use std::path::PathBuf;

#[test]
fn wdb256_tip_out_incremental_must_reborrow_owned_csr_clone_into_demoted_mut_bfs() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let bfs_paths = [
        tip.join("graph_bfs_engine.rs"),
        gen.join("graph/graph_bfs_engine.rs"),
    ];
    let mut demoted = false;
    for path in &bfs_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("bfs");
        if text.contains("fn graph_bfs_run_dense(csr: &mut DenseCsr") {
            demoted = true;
            break;
        }
    }
    assert!(
        demoted,
        "WDB-256: demoted &mut DenseCsr graph_bfs_run_dense formal missing"
    );

    let engine_paths = [
        tip.join("graph_incremental_views.rs"),
        gen.join("graph/graph_incremental_views.rs"),
    ];
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("incremental");
        let bad = text.contains("graph_bfs_run_dense(csr.clone(),")
            || text.contains("graph_sssp_run_dense(csr.clone(),");
        eprintln!("WDB-256 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-256 RED: tip-out/product passes owned csr.clone() into demoted &mut DenseCsr bfs. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-256: graph_incremental_views missing");
}
