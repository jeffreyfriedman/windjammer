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

//! WDB-263: demoted `&Vec` into owned `graph_vertex_i64_init_identity` must clone (WCC).
//!
//! Twin of WDB-259 (CDLP). Tip-out greened with `vertices.clone()`, but gen still emits:
//!   `graph_vertex_i64_init_identity(&vertices)` while formal is `vertices: Vec<i64>`
//! → E0308 expected `Vec`, found `&Vec`.
//! Signature-driven: `vertices.clone()` / move.

use std::path::PathBuf;

#[test]
fn wdb263_tip_out_wcc_must_clone_ref_vec_into_owned_init_identity() {
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
        "WDB-263: owned Vec graph_vertex_i64_init_identity formal missing"
    );

    let engine_paths = [
        tip.join("graph_wcc_engine.rs"),
        gen.join("graph/graph_wcc_engine.rs"),
    ];
    let mut saw = false;
    let mut any_bad = false;
    let mut bad_path = String::new();
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("wcc");
        let bad = text.contains("graph_vertex_i64_init_identity(&vertices)")
            && !text.contains("graph_vertex_i64_init_identity(vertices.clone())");
        eprintln!("WDB-263 bad={} path={}", bad, path.display());
        if bad {
            any_bad = true;
            bad_path = path.display().to_string();
        }
    }
    assert!(saw, "WDB-263: graph_wcc_engine missing");
    assert!(
        !any_bad,
        "WDB-263 RED: tip-out/product passes &Vec into owned init_identity (WCC). {}",
        bad_path
    );
}
