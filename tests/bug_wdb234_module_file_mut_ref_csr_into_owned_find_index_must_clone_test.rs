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

//! WDB-234: `&mut DenseCsr` into owned `DenseCsr` formal must clone (batch engine).
//!
//! Twin of WDB-219 (graph_sql tip-out). Product residual historically in tip-out/gen
//! graph_batch_engine:
//!   `graph_dense_csr_find_index(csr: DenseCsr, …)` called with bare `csr`
//!   while caller formal is `&mut DenseCsr` → E0308.
//! Signature-driven: clone — or GREEN when formal demotes to `&DenseCsr` (`&mut` coerces).

use std::path::PathBuf;

#[test]
fn wdb234_tip_out_batch_must_clone_mut_ref_csr_into_owned_find_index() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let dense_paths = [
        tip.join("graph_dense_csr.rs"),
        gen.join("graph/graph_dense_csr.rs"),
    ];
    let mut formal_kind = "missing";
    for path in &dense_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("dense");
        if text.contains("fn graph_dense_csr_find_index(csr: DenseCsr") {
            formal_kind = "owned";
            break;
        }
        if text.contains("fn graph_dense_csr_find_index(csr: &DenseCsr")
            || text.contains("fn graph_dense_csr_find_index(csr: &mut DenseCsr")
        {
            formal_kind = "borrowed";
            break;
        }
    }
    assert!(
        formal_kind != "missing",
        "WDB-234: find_index DenseCsr formal missing"
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
        // Owned formal requires clone; borrowed formal accepts bare &mut→& coerce.
        let bad = has_mut_helpers
            && formal_kind == "owned"
            && text.contains("graph_dense_csr_find_index(csr,")
            && !text.contains("graph_dense_csr_find_index(csr.clone(),")
            && !text.contains("graph_dense_csr_find_index((*csr).clone(),");
        eprintln!(
            "WDB-234 formal_kind={} has_mut_helpers={} bad={} path={}",
            formal_kind,
            has_mut_helpers,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-234 RED: tip-out/product passes &mut csr into owned find_index without clone. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-234: graph_batch_engine missing");
}
