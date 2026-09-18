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

//! WDB-274: demoted `&DenseCsr` / `&self.csr` into owned `graph_lcc_run_dense` must clone.
//!
//! Twin of WDB-241 (ecs) / inverse of WDB-233 (analytics clone into mut-ref). Tip-out
//! analytics session still emits:
//!   `graph_lcc_run_dense(&self.csr)` while formal is `csr: DenseCsr` → E0308.
//! Signature-driven: `self.csr.clone()` / move when formal is owned.

use std::path::PathBuf;

#[test]
fn wdb274_tip_out_analytics_must_clone_ref_csr_into_owned_lcc_run_dense() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let eng_paths = [
        tip.join("graph_lcc_engine.rs"),
        gen.join("graph/graph_lcc_engine.rs"),
    ];
    let mut owned = false;
    for path in &eng_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("lcc");
        if text.contains("fn graph_lcc_run_dense(csr: DenseCsr") {
            owned = true;
            break;
        }
    }
    assert!(
        owned,
        "WDB-274: owned DenseCsr graph_lcc_run_dense formal missing"
    );

    let session_paths = [
        tip.join("graph_analytics_session.rs"),
        gen.join("graph/graph_analytics_session.rs"),
    ];
    let mut saw = false;
    let mut any_bad = false;
    let mut bad_path = String::new();
    for path in &session_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("session");
        let bad = text.contains("graph_lcc_run_dense(&self.csr)")
            && !text.contains("graph_lcc_run_dense(self.csr.clone())");
        eprintln!("WDB-274 bad={} path={}", bad, path.display());
        if bad {
            any_bad = true;
            bad_path = path.display().to_string();
        }
    }
    assert!(saw, "WDB-274: graph_analytics_session missing");
    assert!(
        !any_bad,
        "WDB-274 RED: tip-out/product passes &DenseCsr into owned graph_lcc_run_dense. {}",
        bad_path
    );
}
