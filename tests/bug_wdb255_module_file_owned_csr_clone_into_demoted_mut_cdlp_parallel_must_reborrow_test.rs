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

//! WDB-255: owned `csr.clone()` into demoted `&mut DenseCsr` CDLP parallel must reborrow.
//!
//! Twin of WDB-233 (analytics). Product residual tip-out/gen graph_cdlp_engine:
//!   `graph_cdlp_run_dense_parallel(csr.clone(), max_iters)`
//! while formal is `&mut DenseCsr` → E0308.
//! Signature-driven: pass `&mut csr`.

use std::path::PathBuf;

#[test]
fn wdb255_tip_out_cdlp_must_reborrow_owned_csr_clone_into_demoted_mut_parallel() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let map_paths = [
        tip.join("graph_cdlp_engine.rs"),
        gen.join("graph/graph_cdlp_engine.rs"),
    ];
    let mut demoted = false;
    for path in &map_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("cdlp");
        if text.contains("fn graph_cdlp_run_dense_parallel(csr: &mut DenseCsr") {
            demoted = true;
            break;
        }
    }
    assert!(
        demoted,
        "WDB-255: demoted &mut DenseCsr graph_cdlp_run_dense_parallel formal missing"
    );

    let mut saw = false;
    for path in &map_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("cdlp");
        let bad = text.contains("graph_cdlp_run_dense_parallel(csr.clone(),");
        eprintln!("WDB-255 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-255 RED: tip-out/product passes owned csr.clone() into demoted &mut DenseCsr parallel. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-255: graph_cdlp_engine missing");
}
