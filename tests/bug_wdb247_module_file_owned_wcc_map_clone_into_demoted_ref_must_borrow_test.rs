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

//! WDB-247: owned `p.clone()` into demoted `&GraphVertexI64Map` get must borrow (WCC).
//!
//! Twin of WDB-222 (BFS). Product residual tip-out/gen graph_wcc_engine:
//!   `graph_vertex_i64_get(p.clone(), …)` / `graph_vertex_i64_set(p.clone(), …)`
//!   while get formal is `&GraphVertexI64Map` → E0308.
//! Signature-driven: pass `&p` / bare `p` for set ownership per signature.

use std::path::PathBuf;

#[test]
fn wdb247_tip_out_wcc_must_borrow_owned_map_clone_into_demoted_get() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let map_paths = [
        tip.join("graph_vertex_map.rs"),
        gen.join("graph/graph_vertex_map.rs"),
    ];
    let mut demoted_get = false;
    for path in &map_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("map");
        if text.contains("fn graph_vertex_i64_get(map: &GraphVertexI64Map") {
            demoted_get = true;
            break;
        }
    }
    assert!(demoted_get, "WDB-247: demoted &GraphVertexI64Map get formal missing");

    // Prefer tip-out when present (gen may lag behind tip multipass).
    let paths = if tip.join("graph_wcc_engine.rs").exists() {
        vec![tip.join("graph_wcc_engine.rs")]
    } else {
        vec![gen.join("graph/graph_wcc_engine.rs")]
    };
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("wcc");
        let bad = text.contains("graph_vertex_i64_get(p.clone(),")
            || text.contains("graph_vertex_i64_get(parent.clone(),");
        eprintln!("WDB-247 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-247 RED: tip-out/product passes owned map.clone() into demoted &GraphVertexI64Map get. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-247: graph_wcc_engine missing");
}
