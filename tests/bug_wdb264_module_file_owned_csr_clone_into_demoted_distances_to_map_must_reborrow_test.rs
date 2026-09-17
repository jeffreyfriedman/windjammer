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

//! WDB-264: owned `csr.clone()` into demoted `&DenseCsr` `distances_to_map` must reborrow (BFS).
//!
//! Twin of WDB-252/235. Tip-out greened to bare `graph_dense_distances_to_map(csr, …)`
//! after demoting formal to `&DenseCsr`, but gen still emits:
//!   `graph_dense_distances_to_map(csr.clone(), distances)` → E0308.
//! Signature-driven: pass `&csr` / bare `csr` when formal is `&DenseCsr`.

use std::path::PathBuf;

#[test]
fn wdb264_tip_out_bfs_must_reborrow_owned_csr_clone_into_demoted_distances_to_map() {
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
        if text.contains("fn graph_dense_distances_to_map(csr: &DenseCsr") {
            demoted = true;
            break;
        }
    }
    assert!(
        demoted,
        "WDB-264: demoted &DenseCsr graph_dense_distances_to_map formal missing"
    );

    let engine_paths = [
        tip.join("graph_bfs_engine.rs"),
        gen.join("graph/graph_bfs_engine.rs"),
    ];
    let mut saw = false;
    let mut any_bad = false;
    let mut bad_path = String::new();
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("bfs");
        let bad = text.contains("graph_dense_distances_to_map(csr.clone(),");
        eprintln!("WDB-264 bad={} path={}", bad, path.display());
        if bad {
            any_bad = true;
            bad_path = path.display().to_string();
        }
    }
    assert!(saw, "WDB-264: graph_bfs_engine missing");
    assert!(
        !any_bad,
        "WDB-264 RED: tip-out/product passes csr.clone() into demoted &DenseCsr distances_to_map. {}",
        bad_path
    );
}
