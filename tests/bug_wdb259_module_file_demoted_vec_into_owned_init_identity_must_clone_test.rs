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

//! WDB-259: demoted `&Vec` into owned `graph_vertex_i64_init_identity` must clone (CDLP).
//!
//! Twin of WDB-241 (ecs `&ids`→owned Vec). Product residual tip-out/gen
//! graph_cdlp_engine:
//!   `graph_vertex_i64_init_identity(vertices: Vec<i64>)`
//!   called as `graph_vertex_i64_init_identity(&vertices)` → E0308.
//! Signature-driven: `vertices.clone()` / move when owned.

use std::path::PathBuf;

#[test]
fn wdb259_tip_out_cdlp_must_clone_ref_vec_into_owned_init_identity() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let map_paths = [
        tip.join("graph_vertex_map.rs"),
        gen.join("graph/graph_vertex_map.rs"),
    ];
    let mut owned = false;
    for path in &map_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("vertex_map");
        if text.contains("fn graph_vertex_i64_init_identity(vertices: Vec<i64>") {
            owned = true;
            break;
        }
    }
    assert!(
        owned,
        "WDB-259: owned Vec graph_vertex_i64_init_identity formal missing"
    );

    let engine_paths = [
        tip.join("graph_cdlp_engine.rs"),
        gen.join("graph/graph_cdlp_engine.rs"),
    ];
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("cdlp");
        let bad = text.contains("graph_vertex_i64_init_identity(&vertices)")
            && !text.contains("graph_vertex_i64_init_identity(vertices.clone())")
            && !text.contains("graph_vertex_i64_init_identity(vertices.to_vec())");
        eprintln!("WDB-259 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-259 RED: tip-out/product passes &Vec into owned init_identity. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-259: graph_cdlp_engine missing");
}
