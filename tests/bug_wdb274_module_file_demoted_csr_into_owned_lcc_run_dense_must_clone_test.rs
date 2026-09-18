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

//! WDB-274: `&self.csr` / demoted `&DenseCsr` vs owned `graph_lcc_run_dense` coherence.
//!
//! Tip multipass may demote readonly `DenseCsr` formals to `&DenseCsr` (field projection
//! only) — then `&self.csr` is correct. When the formal stays owned `DenseCsr`, the call
//! must clone (`self.csr.clone()`). Prefer tip-out over stale gen (WDB-236/273 pattern).

use std::path::PathBuf;

#[test]
fn wdb274_tip_out_analytics_must_clone_ref_csr_into_owned_lcc_run_dense() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    // Prefer tip-out when present — gen lag must not poison tip truth.
    let eng_paths = if tip.join("graph_lcc_engine.rs").exists() {
        vec![tip.join("graph_lcc_engine.rs")]
    } else if tip.join("graph/graph_lcc_engine.rs").exists() {
        vec![tip.join("graph/graph_lcc_engine.rs")]
    } else {
        vec![gen.join("graph/graph_lcc_engine.rs")]
    };
    let mut owned = false;
    let mut demoted = false;
    for path in &eng_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("lcc");
        if text.contains("fn graph_lcc_run_dense(csr: DenseCsr") {
            owned = true;
            break;
        }
        if text.contains("fn graph_lcc_run_dense(csr: &DenseCsr") {
            demoted = true;
            break;
        }
    }
    assert!(
        owned || demoted,
        "WDB-274: graph_lcc_run_dense DenseCsr formal missing"
    );

    let session_paths = if tip.join("graph_analytics_session.rs").exists() {
        vec![tip.join("graph_analytics_session.rs")]
    } else if tip.join("graph/graph_analytics_session.rs").exists() {
        vec![tip.join("graph/graph_analytics_session.rs")]
    } else {
        vec![gen.join("graph/graph_analytics_session.rs")]
    };
    let mut saw = false;
    for path in &session_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("session");
        if owned {
            let bad = text.contains("graph_lcc_run_dense(&self.csr)")
                && !text.contains("graph_lcc_run_dense(self.csr.clone())");
            eprintln!("WDB-274 owned formal bad={} path={}", bad, path.display());
            assert!(
                !bad,
                "WDB-274 RED: tip-out passes &DenseCsr into owned graph_lcc_run_dense. {}",
                path.display()
            );
        } else {
            // Demoted formal: `&self.csr` is correct; owned clone would be wrong.
            let bad = text.contains("graph_lcc_run_dense(self.csr.clone())");
            eprintln!("WDB-274 demoted formal bad={} path={}", bad, path.display());
            assert!(
                !bad,
                "WDB-274 RED: tip-out clones into demoted &DenseCsr graph_lcc_run_dense. {}",
                path.display()
            );
        }
    }
    assert!(saw, "WDB-274: graph_analytics_session missing");
}
