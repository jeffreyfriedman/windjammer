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

//! WDB-235: `&mut DenseCsr` into owned `graph_dense_distances_to_map` must clone.
//!
//! Twin of WDB-234 (`find_index`). Product residual still in tip-out/gen
//! graph_batch_engine (~13× DenseCsr←&mut overall):
//!   `graph_dense_distances_to_map(csr: DenseCsr, …)` called with bare `csr`
//!   while caller formal is `&mut DenseCsr` → E0308.
//! Signature-driven: clone (or demote distances_to_map to `&DenseCsr`).

use std::path::PathBuf;

#[test]
fn wdb235_tip_out_batch_must_clone_mut_ref_csr_into_owned_distances_to_map() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let dense_paths = [
        tip.join("graph_dense_csr.rs"),
        gen.join("graph/graph_dense_csr.rs"),
    ];
    let mut owned_formal = false;
    for path in &dense_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("dense");
        if text.contains("fn graph_dense_distances_to_map(csr: DenseCsr") {
            owned_formal = true;
            break;
        }
    }
    assert!(
        owned_formal,
        "WDB-235: owned DenseCsr distances_to_map formal missing"
    );

    let paths = [
        tip.join("graph_batch_engine.rs"),
        gen.join("graph/graph_batch_engine.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("batch");
        let has_mut_helpers = text.contains("csr: &mut DenseCsr");
        let bad = has_mut_helpers
            && text.contains("graph_dense_distances_to_map(csr,")
            && !text.contains("graph_dense_distances_to_map(csr.clone(),")
            && !text.contains("graph_dense_distances_to_map((*csr).clone(),");
        eprintln!(
            "WDB-235 has_mut_helpers={} bad={} path={}",
            has_mut_helpers,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-235 RED: tip-out/product passes &mut csr into owned distances_to_map without clone. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-235: graph_batch_engine missing");
}
