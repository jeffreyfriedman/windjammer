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

//! WDB-253: owned `distances.clone()` into demoted `&GraphVertexI64Map` contains/len
//! must borrow (BFS).
//!
//! Twin of WDB-222 (get). Product residual tip-out/gen graph_bfs_engine:
//!   `graph_vertex_i64_contains(distances.clone(), …)`
//!   `graph_vertex_i64_len(distances.clone())`
//! while formals are `&GraphVertexI64Map` → E0308.
//! Signature-driven: pass `&distances`.

use std::path::PathBuf;

#[test]
fn wdb253_tip_out_bfs_must_borrow_owned_map_clone_into_demoted_contains_len() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let map_paths = [
        tip.join("graph_vertex_map.rs"),
        gen.join("graph/graph_vertex_map.rs"),
    ];
    let mut demoted_contains = false;
    let mut demoted_len = false;
    let mut owned_len = false;
    for path in &map_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("vertex_map");
        if text.contains("fn graph_vertex_i64_contains(map: &GraphVertexI64Map") {
            demoted_contains = true;
        }
        if text.contains("fn graph_vertex_i64_len(map: &GraphVertexI64Map") {
            demoted_len = true;
        }
        if text.contains("fn graph_vertex_i64_len(map: GraphVertexI64Map") {
            owned_len = true;
        }
    }
    assert!(
        demoted_contains || demoted_len || owned_len,
        "WDB-253: graph_vertex_i64 contains/len formals missing"
    );

    let engine_paths = [
        tip.join("graph_bfs_engine.rs"),
        gen.join("graph/graph_bfs_engine.rs"),
    ];
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("bfs");
        // Only flag clone into demoted slots — tip may keep len Owned (+ clone).
        let bad = (demoted_contains
            && (text.contains("graph_vertex_i64_contains(distances.clone(),")
                || text.contains("graph_vertex_i64_contains(self.distances.clone(),")))
            || (demoted_len
                && (text.contains("graph_vertex_i64_len(distances.clone())")
                    || text.contains("graph_vertex_i64_len(self.distances.clone())")));
        eprintln!(
            "WDB-253 demoted_contains={} demoted_len={} owned_len={} bad={} path={}",
            demoted_contains,
            demoted_len,
            owned_len,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-253 RED: tip-out/product passes owned map.clone() into demoted contains/len. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-253: graph_bfs_engine missing");
}
