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

//! WDB-276: demoted `&Vec<u32>` into owned `graph_par_bfs_bind` must clone.
//!
//! Twin of WDB-261 (simd LCC bind). Tip-out BFS parallel still emits:
//!   `graph_par_bfs_bind(&out.offsets, &out.neighbors, &inn.in_offsets, &inn.in_neighbors, …)`
//! while formals are owned `Vec<u32>` → E0308.
//! Signature-driven: clone offsets/neighbors (or demote FFI formals to `&[u32]`).

use std::path::PathBuf;

#[test]
fn wdb276_tip_out_bfs_must_clone_ref_vec_into_owned_par_bfs_bind() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let port_paths = if tip.join("graph_parallel_port.rs").exists() {
        vec![tip.join("graph_parallel_port.rs")]
    } else if tip.join("graph/graph_parallel_port.rs").exists() {
        vec![tip.join("graph/graph_parallel_port.rs")]
    } else {
        vec![gen.join("graph/graph_parallel_port.rs")]
    };
    let mut owned = false;
    for path in &port_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("parallel");
        if text.contains("fn graph_par_bfs_bind(offsets: Vec<u32>") {
            owned = true;
            break;
        }
    }
    assert!(
        owned,
        "WDB-276: owned Vec graph_par_bfs_bind formal missing (must not demote into owned FFI)"
    );

    let engine_paths = if tip.join("graph_bfs_engine.rs").exists() {
        vec![tip.join("graph_bfs_engine.rs")]
    } else if tip.join("graph/graph_bfs_engine.rs").exists() {
        vec![tip.join("graph/graph_bfs_engine.rs")]
    } else {
        vec![gen.join("graph/graph_bfs_engine.rs")]
    };
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("bfs");
        let bad = text.contains(
            "graph_par_bfs_bind(&out.offsets, &out.neighbors, &inn.in_offsets, &inn.in_neighbors,",
        );
        eprintln!("WDB-276 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-276 RED: tip-out/product passes &Vec into owned graph_par_bfs_bind. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-276: graph_bfs_engine missing");
}
