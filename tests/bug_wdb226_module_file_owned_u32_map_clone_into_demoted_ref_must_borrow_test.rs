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

//! WDB-226: owned `GraphVertexU32Map` / `.clone()` into demoted `&GraphVertexU32Map` must borrow.
//!
//! Product residual (~6×), tip-out/gen graph_cdlp_engine:
//!   `graph_vertex_u32_get(map: &GraphVertexU32Map, …)` called with
//!   `counts.clone()` / `seen.clone()` → expected `&`, found owned.
//! Twin of WDB-222/223. Signature-driven.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str = include_str!(
    "fixtures/library_multipass/wdb226_owned_u32_map_clone_into_demoted_ref_must_borrow.wj"
);

#[test]
fn wdb226_codegen_owned_u32_map_into_demoted_ref_must_borrow() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let demoted = rs.contains("fn get(map: &GraphVertexU32Map")
        || rs.contains("get(map: &GraphVertexU32Map");
    if demoted {
        assert!(
            rs.contains("get(&map") || rs.contains("get(&map.clone()"),
            "WDB-226: demoted &U32Map formal must borrow at call site. Generated:\n{rs}"
        );
        assert!(
            !rs.contains("get(map.clone()"),
            "WDB-226: no owned clone into &U32Map. Generated:\n{rs}"
        );
    }
    assert!(ok, "WDB-226 fixture must cargo-check. Generated:\n{rs}");
}

use std::path::PathBuf;

#[test]
fn wdb226_tip_out_cdlp_must_borrow_owned_u32_map_into_demoted_get() {
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
        if text.contains("fn graph_vertex_u32_get(map: &GraphVertexU32Map") {
            demoted = true;
            break;
        }
    }
    assert!(demoted, "WDB-226: demoted &GraphVertexU32Map get formal missing");

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
        let bad = text.contains("graph_vertex_u32_get(counts.clone(),")
            || text.contains("graph_vertex_u32_get(seen.clone(),");
        eprintln!("WDB-226 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-226 RED: tip-out/product passes owned u32 map.clone() into demoted &GraphVertexU32Map. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-226: graph_cdlp_engine missing");
}
