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

//! WDB-283: demoted `&csr` into owned `graph_bfs_run_dense` must clone (incremental).
//!
//! Twin of WDB-241; inverse of WDB-268 (clone into demoted). Tip-out
//! graph_incremental_views still emits:
//!   `graph_bfs_run_dense(&csr, source)` while formal is `csr: DenseCsr` → E0308.
//! Signature-driven: `csr.clone()` / move when formal is owned.

use std::path::PathBuf;

#[test]
fn wdb283_tip_out_incremental_must_clone_ref_csr_into_owned_bfs_run_dense() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let eng_paths = [
        tip.join("graph_bfs_engine.rs"),
        gen.join("graph/graph_bfs_engine.rs"),
    ];
    let mut owned = false;
    for path in &eng_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("bfs");
        if text.contains("fn graph_bfs_run_dense(csr: DenseCsr") {
            owned = true;
            break;
        }
    }
    assert!(
        owned,
        "WDB-283: owned DenseCsr graph_bfs_run_dense formal missing"
    );

    let view_paths = if tip.join("graph_incremental_views.rs").exists() {
        vec![tip.join("graph_incremental_views.rs")]
    } else {
        vec![gen.join("graph/graph_incremental_views.rs")]
    };
    let mut saw = false;
    for path in &view_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("incremental");
        let bad = text.contains("graph_bfs_run_dense(&csr,")
            && !text.contains("graph_bfs_run_dense(csr.clone(),")
            && !text.contains("graph_bfs_run_dense(csr,");
        eprintln!("WDB-283 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-283 RED: tip-out/product passes &DenseCsr into owned graph_bfs_run_dense. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-283: graph_incremental_views missing");
}
