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

//! WDB-223: owned `GraphVertexF64Map` / `.clone()` into demoted `&GraphVertexF64Map` must borrow.
//!
//! Product residual (~10×), tip-out/gen graph_pagerank_engine:
//!   `graph_vertex_f64_get(map: &GraphVertexF64Map, …)` called with
//!   `self.scores.clone()` / `scores.clone()` → expected `&`, found owned.
//! Twin of WDB-222 (I64Map). Signature-driven.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str = include_str!(
    "fixtures/library_multipass/wdb223_owned_f64_map_clone_into_demoted_ref_must_borrow.wj"
);

#[test]
fn wdb223_codegen_owned_f64_map_into_demoted_ref_must_borrow() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let demoted = rs.contains("fn get(map: &GraphVertexF64Map")
        || rs.contains("get(map: &GraphVertexF64Map");
    if demoted {
        assert!(
            rs.contains("get(&map") || rs.contains("get(&map.clone()"),
            "WDB-223: demoted &F64Map formal must borrow at call site. Generated:\n{rs}"
        );
        assert!(
            !rs.contains("get(map.clone()"),
            "WDB-223: no owned clone into &F64Map. Generated:\n{rs}"
        );
    }
    assert!(ok, "WDB-223 fixture must cargo-check. Generated:\n{rs}");
}

use std::path::PathBuf;

#[test]
fn wdb223_tip_out_pagerank_must_borrow_owned_f64_map_into_demoted_get() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    // Prefer tip-out when present (gen may lag behind tip multipass).
    let map_path = if tip.join("graph_vertex_map.rs").exists() {
        tip.join("graph_vertex_map.rs")
    } else {
        gen.join("graph/graph_vertex_map.rs")
    };
    assert!(map_path.exists(), "WDB-223: graph_vertex_map missing");
    let map_text = std::fs::read_to_string(&map_path).expect("vertex_map");
    let demoted_get = map_text.contains("fn graph_vertex_f64_get(map: &GraphVertexF64Map");
    let demoted_sum = map_text.contains("fn graph_vertex_f64_sum(map: &GraphVertexF64Map");
    assert!(
        demoted_get,
        "WDB-223: demoted &GraphVertexF64Map get formal missing"
    );

    let engine_paths = if tip.join("graph_pagerank_engine.rs").exists() {
        vec![tip.join("graph_pagerank_engine.rs")]
    } else {
        vec![gen.join("graph/graph_pagerank_engine.rs")]
    };
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("pagerank");
        let bad_get = demoted_get
            && (text.contains("graph_vertex_f64_get(self.scores.clone(),")
                || text.contains("graph_vertex_f64_get(scores.clone(),"));
        // Tip may keep sum Owned (`map: GraphVertexF64Map`) — clone is then required.
        let bad_sum = demoted_sum
            && (text.contains("graph_vertex_f64_sum(self.scores.clone(),")
                || text.contains("graph_vertex_f64_sum(scores.clone(),"));
        let bad = bad_get || bad_sum;
        eprintln!(
            "WDB-223 demoted_get={} demoted_sum={} bad={} path={}",
            demoted_get,
            demoted_sum,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-223 RED: tip-out/product passes owned f64 map.clone() into demoted &GraphVertexF64Map. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-223: graph_pagerank_engine missing");
}
