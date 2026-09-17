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

//! WDB-254: owned `csr.clone()` into demoted `&DenseCsr` SQL edge-count helpers
//! must reborrow (datafusion).
//!
//! Twin of WDB-252 (vertex_count/find_index) / WDB-248 (analytics). Tip demotes
//! `graph_sql_csr_count_edges_*` to `&DenseCsr`, but tip-out/gen still emits:
//!   `graph_sql_csr_count_edges_src_in_and_weight_gte(csr.clone(), …)`
//!   `graph_sql_csr_count_edges_weight_gte(csr.clone(), …)`
//! → E0308 expected `&DenseCsr`, found `DenseCsr`.
//! Signature-driven: pass `&csr`.

use std::path::PathBuf;

#[test]
fn wdb254_tip_out_datafusion_must_reborrow_owned_csr_clone_into_demoted_edge_counts() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let query_paths = [
        tip.join("graph_sql_query_port.rs"),
        gen.join("graph/graph_sql_query_port.rs"),
    ];
    let mut demoted = false;
    for path in &query_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("query_port");
        if text.contains("fn graph_sql_csr_count_edges_src_in_and_weight_gte(csr: &DenseCsr")
            || text.contains("fn graph_sql_csr_count_edges_weight_gte(csr: &DenseCsr")
        {
            demoted = true;
            break;
        }
    }
    assert!(
        demoted,
        "WDB-254: demoted &DenseCsr count_edges formal missing"
    );

    let engine_paths = [
        tip.join("graph_sql_datafusion_port.rs"),
        gen.join("graph/graph_sql_datafusion_port.rs"),
    ];
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("datafusion");
        let bad = text
            .contains("graph_sql_csr_count_edges_src_in_and_weight_gte(csr.clone(),")
            || text.contains("graph_sql_csr_count_edges_weight_gte(csr.clone(),")
            || text.contains("graph_sql_csr_count_edges_src_in(csr.clone(),")
            || text.contains("graph_sql_csr_count_edges_weight_gte_src_in(csr.clone(),");
        eprintln!("WDB-254 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-254 RED: tip-out/product passes owned csr.clone() into demoted count_edges. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-254: graph_sql_datafusion_port missing");
}
