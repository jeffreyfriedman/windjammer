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

//! WDB-250: owned `labels.clone()` into demoted `&GraphVertexI64Map` get must borrow (CDLP).
//!
//! Twin of WDB-222 (BFS) / WDB-247 (WCC). Product residual tip-out/gen graph_cdlp_engine:
//!   `graph_vertex_i64_get(labels.clone(), …)` while get formal is
//!   `&GraphVertexI64Map` → E0308 expected `&`, found owned.
//! (WDB-226 already gates CDLP u32 map clones; this gates the i64 labels path.)
//! Signature-driven: pass `&labels` / bare borrow.

use std::path::PathBuf;

#[test]
fn wdb250_tip_out_cdlp_must_borrow_owned_i64_map_clone_into_demoted_get() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let map_paths = [
        tip.join("graph_vertex_map.rs"),
        gen.join("graph/graph_vertex_map.rs"),
    ];
    let mut demoted = false;
    for path in &map_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("vertex_map");
        if text.contains("fn graph_vertex_i64_get(map: &GraphVertexI64Map") {
            demoted = true;
            break;
        }
    }
    assert!(demoted, "WDB-250: demoted &GraphVertexI64Map get formal missing");

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
        let bad = text.contains("graph_vertex_i64_get(labels.clone(),")
            || text.contains("graph_vertex_i64_get(self.labels.clone(),");
        eprintln!("WDB-250 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-250 RED: tip-out/product passes owned i64 map.clone() into demoted &GraphVertexI64Map. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-250: graph_cdlp_engine missing");
}
