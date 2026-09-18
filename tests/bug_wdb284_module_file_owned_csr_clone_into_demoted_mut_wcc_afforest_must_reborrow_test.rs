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

//! WDB-284: gen `csr.clone()` into demoted `&mut DenseCsr` WCC afforest must reborrow.
//!
//! Twin of WDB-255/273. Tip greened to `graph_wcc_run_dense_afforest(csr)` with
//! formal `&mut DenseCsr`, but gen still emits
//!   `graph_wcc_run_dense_afforest(csr.clone())` → E0308.
//! Signature-driven: pass bare/`&mut csr`; sync tip-out→gen.

use std::path::PathBuf;

#[test]
fn wdb284_gen_wcc_must_reborrow_owned_csr_clone_into_demoted_mut_afforest() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_wcc_engine.rs"),
        gen.join("graph/graph_wcc_engine.rs"),
    ];
    let mut demoted = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("wcc");
        if text.contains("fn graph_wcc_run_dense_afforest(csr: &mut DenseCsr") {
            demoted = true;
            break;
        }
    }
    assert!(
        demoted,
        "WDB-284: demoted &mut DenseCsr graph_wcc_run_dense_afforest formal missing"
    );

    // Prefer gen residual (tip greened); fail if either tree still clones.
    let mut saw = false;
    let mut any_bad = false;
    let mut bad_path = String::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("wcc");
        let bad = text.contains("graph_wcc_run_dense_afforest(csr.clone())");
        eprintln!("WDB-284 bad={} path={}", bad, path.display());
        if bad {
            any_bad = true;
            bad_path = path.display().to_string();
        }
    }
    assert!(saw, "WDB-284: graph_wcc_engine missing");
    assert!(
        !any_bad,
        "WDB-284 RED: tip-out/product passes owned csr.clone() into demoted &mut DenseCsr afforest. {}",
        bad_path
    );
}
