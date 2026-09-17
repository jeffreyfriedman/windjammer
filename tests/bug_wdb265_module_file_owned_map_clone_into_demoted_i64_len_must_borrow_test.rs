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

//! WDB-265: owned `distances.clone()` into demoted `&GraphVertexI64Map` `i64_len` must borrow (BFS).
//!
//! Gen demoted `graph_vertex_i64_len(map: &GraphVertexI64Map)` but still emits:
//!   `graph_vertex_i64_len(distances.clone())` → E0308.
//! Tip may still keep owned `map: GraphVertexI64Map` (clone then required).
//! Signature-driven: when formal is `&Map`, pass `&distances` / bare `distances`.

use std::path::PathBuf;

#[test]
fn wdb265_tip_out_bfs_must_borrow_owned_map_clone_into_demoted_i64_len() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");

    let pairs = [
        (tip.join("graph_vertex_map.rs"), tip.join("graph_bfs_engine.rs")),
        (
            gen.join("graph/graph_vertex_map.rs"),
            gen.join("graph/graph_bfs_engine.rs"),
        ),
    ];
    let mut saw = false;
    let mut any_bad = false;
    let mut bad_path = String::new();
    for (map_path, engine_path) in &pairs {
        if !map_path.exists() || !engine_path.exists() {
            continue;
        }
        saw = true;
        let map_text = std::fs::read_to_string(map_path).expect("vertex_map");
        let demoted = map_text.contains("fn graph_vertex_i64_len(map: &GraphVertexI64Map");
        let engine = std::fs::read_to_string(engine_path).expect("bfs");
        let bad = demoted && engine.contains("graph_vertex_i64_len(distances.clone())");
        eprintln!(
            "WDB-265 demoted={} bad={} path={}",
            demoted,
            bad,
            engine_path.display()
        );
        if bad {
            any_bad = true;
            bad_path = engine_path.display().to_string();
        }
    }
    assert!(saw, "WDB-265: graph_bfs_engine / vertex_map missing");
    assert!(
        !any_bad,
        "WDB-265 RED: tip-out/product passes distances.clone() into demoted &GraphVertexI64Map i64_len. {}",
        bad_path
    );
}
