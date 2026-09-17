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

//! WDB-260: demoted `&Vec` into owned `graph_vertex_f64_sum` vertices must clone (PageRank).
//!
//! Twin of WDB-241/259. Product residual tip-out/gen graph_pagerank_engine:
//!   `PageRankEngine::sum(&self, vertices: &Vec<i64>)` calls
//!   `graph_vertex_f64_sum(…, vertices: Vec<i64>)` with bare `vertices` (`&Vec`)
//! → E0308 expected `Vec`, found `&Vec`.
//! Signature-driven: `vertices.clone()` into owned formal.

use std::path::PathBuf;

#[test]
fn wdb260_tip_out_pagerank_must_clone_ref_vec_into_owned_f64_sum_vertices() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let map_paths = [
        tip.join("graph_vertex_map.rs"),
        gen.join("graph/graph_vertex_map.rs"),
    ];
    let mut owned_vertices = false;
    for path in &map_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("vertex_map");
        // Match either owned or demoted map formal; vertices must be owned Vec
        if text.contains("fn graph_vertex_f64_sum(map: GraphVertexF64Map, vertices: Vec<i64>")
            || text.contains("fn graph_vertex_f64_sum(map: &GraphVertexF64Map, vertices: Vec<i64>")
        {
            owned_vertices = true;
            break;
        }
    }
    assert!(
        owned_vertices,
        "WDB-260: owned Vec vertices formal on graph_vertex_f64_sum missing"
    );

    let engine_paths = [
        tip.join("graph_pagerank_engine.rs"),
        gen.join("graph/graph_pagerank_engine.rs"),
    ];
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("pagerank");
        // Method sum takes &Vec and forwards bare `vertices` into owned formal
        let has_demoted_sum = text.contains("pub fn sum(&self, vertices: &Vec<i64>)")
            || text.contains("fn sum(&self, vertices: &Vec<i64>)");
        let bad = has_demoted_sum
            && text.contains("graph_vertex_f64_sum(self.scores.clone(), vertices)")
            && !text.contains("graph_vertex_f64_sum(self.scores.clone(), vertices.clone())");
        eprintln!(
            "WDB-260 demoted_sum={} bad={} path={}",
            has_demoted_sum,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-260 RED: tip-out/product passes &Vec into owned f64_sum vertices. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-260: graph_pagerank_engine missing");
}
