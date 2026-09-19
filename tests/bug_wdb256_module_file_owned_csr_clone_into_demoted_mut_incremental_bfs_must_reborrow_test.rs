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

//! WDB-256: `csr` into `graph_bfs_run_dense` must match formal ownership (incremental).
//!
//! Tip historically demoted `&mut DenseCsr` while emitting `csr.clone()`. Tip now
//! keeps owned `csr: DenseCsr` + `csr.clone()` — correct. Gate accepts owned+clone
//! or demoted+reborrow.

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
    let mut demoted_mut = false;
    let mut owned = false;
    for path in &bfs_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("bfs");
        if text.contains("fn graph_bfs_run_dense(csr: &mut DenseCsr") {
            demoted_mut = true;
            break;
        }
        if text.contains("fn graph_bfs_run_dense(csr: DenseCsr") {
            owned = true;
            break;
        }
    }
    assert!(
        demoted_mut || owned,
        "WDB-256: graph_bfs_run_dense DenseCsr formal missing (owned or &mut)"
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
        let bad = if demoted_mut {
            text.contains("graph_bfs_run_dense(csr.clone(),")
                || text.contains("graph_sssp_run_dense(csr.clone(),")
        } else {
            // Owned: &mut / & into owned is RED.
            text.contains("graph_bfs_run_dense(&mut csr,")
                || text.contains("graph_bfs_run_dense(&csr,")
        };
        eprintln!(
            "WDB-256 demoted_mut={} owned={} bad={} path={}",
            demoted_mut,
            owned,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-256 RED: tip-out/product call-site ownership mismatches bfs_run_dense formal. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-256: graph_incremental_views missing");
}
