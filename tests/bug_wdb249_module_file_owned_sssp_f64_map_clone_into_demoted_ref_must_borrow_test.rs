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

//! WDB-249: owned `distances.clone()` into demoted `&GraphVertexF64Map` get must borrow (SSSP).
//!
//! Twin of WDB-223 (PageRank). Product residual tip-out/gen graph_sssp_engine:
//!   `graph_vertex_f64_get(distances.clone(), …)` while get formal is
//!   `&GraphVertexF64Map` → E0308 expected `&`, found owned.
//! Signature-driven: pass `&distances` / bare borrow.

use std::path::PathBuf;

#[test]
fn wdb249_tip_out_sssp_must_borrow_owned_f64_map_clone_into_demoted_get() {
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
        if text.contains("fn graph_vertex_f64_get(map: &GraphVertexF64Map") {
            demoted = true;
            break;
        }
    }
    assert!(demoted, "WDB-249: demoted &GraphVertexF64Map get formal missing");

    // Prefer tip-out when present (gen may lag behind tip multipass).
    let engine_paths = if tip.join("graph_sssp_engine.rs").exists() {
        vec![tip.join("graph_sssp_engine.rs")]
    } else {
        vec![gen.join("graph/graph_sssp_engine.rs")]
    };
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("sssp");
        let bad = text.contains("graph_vertex_f64_get(distances.clone(),")
            || text.contains("graph_vertex_f64_get(self.distances.clone(),");
        eprintln!("WDB-249 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-249 RED: tip-out/product passes owned f64 map.clone() into demoted &GraphVertexF64Map. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-249: graph_sssp_engine missing");
}
