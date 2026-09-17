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

//! WDB-267: owned `scores.clone()` into demoted `&GraphVertexF64Map` `f64_sum` must borrow (PageRank).
//!
//! Gen demoted `graph_vertex_f64_sum(map: &GraphVertexF64Map, …)` but still emits:
//!   `graph_vertex_f64_sum(self.scores.clone(), vertices)` → E0308.
//! Tip may keep owned `map: GraphVertexF64Map` (clone then required).
//! Signature-driven: when formal is `&Map`, pass `&self.scores` / bare `self.scores`.

use std::path::PathBuf;

#[test]
fn wdb267_tip_out_pagerank_must_borrow_owned_map_clone_into_demoted_f64_sum() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");

    let pairs = [
        (
            tip.join("graph_vertex_map.rs"),
            tip.join("graph_pagerank_engine.rs"),
        ),
        (
            gen.join("graph/graph_vertex_map.rs"),
            gen.join("graph/graph_pagerank_engine.rs"),
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
        let demoted = map_text.contains("fn graph_vertex_f64_sum(map: &GraphVertexF64Map");
        let engine = std::fs::read_to_string(engine_path).expect("pagerank");
        let bad = demoted
            && (engine.contains("graph_vertex_f64_sum(self.scores.clone(),")
                || engine.contains("graph_vertex_f64_sum(scores.clone(),")
                || engine.contains("graph_vertex_f64_sum(engine.scores.clone(),"));
        eprintln!(
            "WDB-267 demoted={} bad={} path={}",
            demoted,
            bad,
            engine_path.display()
        );
        if bad {
            any_bad = true;
            bad_path = engine_path.display().to_string();
        }
    }
    assert!(saw, "WDB-267: graph_pagerank_engine / vertex_map missing");
    assert!(
        !any_bad,
        "WDB-267 RED: tip-out/product passes scores.clone() into demoted &GraphVertexF64Map f64_sum. {}",
        bad_path
    );
}
