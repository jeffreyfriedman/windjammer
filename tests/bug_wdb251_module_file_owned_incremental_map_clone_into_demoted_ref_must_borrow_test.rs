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

//! WDB-251: owned `prior.distances/labels.clone()` into demoted `&GraphVertex*Map` get
//! must borrow (incremental views).
//!
//! Twin of WDB-222/223/249/250. Product residual tip-out/gen graph_incremental_views:
//!   `graph_vertex_i64_get(prior.distances.clone(), …)`
//!   `graph_vertex_i64_get(prior.labels.clone(), …)`
//! while get formals are `&GraphVertexI64Map` → E0308.
//! Signature-driven: pass `&prior.distances` / `&prior.labels`.

use std::path::PathBuf;

#[test]
fn wdb251_tip_out_incremental_must_borrow_owned_map_clone_into_demoted_get() {
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
    assert!(demoted, "WDB-251: demoted &GraphVertexI64Map get formal missing");

    let engine_paths = [
        tip.join("graph_incremental_views.rs"),
        gen.join("graph/graph_incremental_views.rs"),
    ];
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("incremental");
        let bad = text.contains("graph_vertex_i64_get(prior.distances.clone(),")
            || text.contains("graph_vertex_i64_get(prior.labels.clone(),");
        eprintln!("WDB-251 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-251 RED: tip-out/product passes owned map.clone() into demoted &GraphVertexI64Map. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-251: graph_incremental_views missing");
}
