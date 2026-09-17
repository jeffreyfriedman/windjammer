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

//! WDB-252: owned `csr.clone()` into demoted `&DenseCsr` vertex_count / find_index
//! must reborrow.
//!
//! Twin of WDB-233/248 (analytics multi_source). After tip demotes
//! `graph_dense_csr_vertex_count` / `graph_dense_csr_find_index` to `&DenseCsr`,
//! tip-out/gen still emits:
//!   `graph_dense_csr_vertex_count(csr.clone())` (bfs/wcc/datafusion)
//!   `graph_dense_csr_vertex_count(self.published.clone())` (epoch_store)
//!   `graph_dense_csr_find_index(csr.clone(), …)` (sssp)
//! → E0308 expected `&DenseCsr`, found `DenseCsr`.
//! Signature-driven: pass `&csr` / `&self.published`.

use std::path::PathBuf;

#[test]
fn wdb252_tip_out_must_reborrow_owned_csr_clone_into_demoted_vertex_count_find_index() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let csr_paths = [
        tip.join("graph_dense_csr.rs"),
        gen.join("graph/graph_dense_csr.rs"),
    ];
    let mut demoted = false;
    for path in &csr_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("dense_csr");
        if text.contains("fn graph_dense_csr_vertex_count(csr: &DenseCsr")
            || text.contains("fn graph_dense_csr_find_index(csr: &DenseCsr")
        {
            demoted = true;
            break;
        }
    }
    assert!(
        demoted,
        "WDB-252: demoted &DenseCsr vertex_count/find_index formal missing"
    );

    let engine_paths = [
        tip.join("graph_bfs_engine.rs"),
        tip.join("graph_wcc_engine.rs"),
        tip.join("graph_sssp_engine.rs"),
        tip.join("graph_epoch_store.rs"),
        tip.join("graph_sql_datafusion_port.rs"),
        gen.join("graph/graph_bfs_engine.rs"),
        gen.join("graph/graph_wcc_engine.rs"),
        gen.join("graph/graph_sssp_engine.rs"),
        gen.join("graph/graph_epoch_store.rs"),
        gen.join("graph/graph_sql_datafusion_port.rs"),
    ];
    let mut saw = false;
    let mut any_bad = false;
    let mut bad_path = String::new();
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("engine");
        let bad = text.contains("graph_dense_csr_vertex_count(csr.clone())")
            || text.contains("graph_dense_csr_vertex_count(self.published.clone())")
            || text.contains("graph_dense_csr_find_index(csr.clone(),");
        eprintln!("WDB-252 bad={} path={}", bad, path.display());
        if bad {
            any_bad = true;
            bad_path = path.display().to_string();
        }
    }
    assert!(saw, "WDB-252: tip-out/gen engine files missing");
    assert!(
        !any_bad,
        "WDB-252 RED: tip-out/product passes owned csr.clone() into demoted &DenseCsr vertex_count/find_index. {}",
        bad_path
    );
}
