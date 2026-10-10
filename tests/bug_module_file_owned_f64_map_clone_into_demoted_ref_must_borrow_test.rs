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

//! DB-223: owned `GraphVertexF64Map` / `.clone()` into demoted `&GraphVertexF64Map` must borrow.
//!
//! Product residual (~10×), tip-out/gen graph_pagerank_engine:
//!   `graph_vertex_f64_get(map: &GraphVertexF64Map, …)` called with
//!   `self.scores.clone()` / `scores.clone()` → expected `&`, found owned.
//! Twin of DB-222 (I64Map). Signature-driven.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str = include_str!(
    "fixtures/library_multipass/owned_f64_map_clone_into_demoted_ref_must_borrow.wj"
);

#[test]
fn codegen_owned_f64_map_into_demoted_ref_must_borrow() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let demoted = rs.contains("fn get(map: &GraphVertexF64Map")
        || rs.contains("get(map: &GraphVertexF64Map");
    if demoted {
        assert!(
            rs.contains("get(&map") || rs.contains("get(&map.clone()"),
            "demoted &F64Map formal must borrow at call site. Generated:\n{rs}"
        );
        assert!(
            !rs.contains("get(map.clone()"),
            "no owned clone into &F64Map. Generated:\n{rs}"
        );
    }
    assert!(ok, "fixture must cargo-check. Generated:\n{rs}");
}
