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

//! WDB-222: owned `GraphVertexI64Map` / `.clone()` into demoted `&GraphVertexI64Map` must borrow.
//!
//! Product residual (~18×), tip-out/gen graph_bfs_engine:
//!   `graph_vertex_i64_get(map: &GraphVertexI64Map, …)` called with
//!   `self.distances.clone()` / `distances.clone()` → expected `&`, found owned.
//! Twin of WDB-212/218. Signature-driven.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str = include_str!(
    "fixtures/library_multipass/wdb222_owned_map_clone_into_demoted_ref_must_borrow.wj"
);

#[test]
fn wdb222_codegen_owned_map_into_demoted_ref_must_borrow() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let demoted = rs.contains("fn get(map: &GraphVertexI64Map")
        || rs.contains("get(map: &GraphVertexI64Map");
    if demoted {
        assert!(
            rs.contains("get(&map") || rs.contains("get(&map.clone()"),
            "WDB-222: demoted &Map formal must borrow at call site. Generated:\n{rs}"
        );
        assert!(
            !rs.contains("get(map.clone()"),
            "WDB-222: no owned clone into &Map. Generated:\n{rs}"
        );
    }
    assert!(ok, "WDB-222 fixture must cargo-check. Generated:\n{rs}");
}

use std::path::PathBuf;

#[test]
fn wdb222_tip_out_bfs_must_borrow_owned_map_into_demoted_get() {
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
    assert!(demoted, "WDB-222: demoted &GraphVertexI64Map get formal missing");

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
        let bad = text.contains("graph_vertex_i64_get(self.distances.clone(),")
            || text.contains("graph_vertex_i64_get(distances.clone(),");
        eprintln!("WDB-222 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-222 RED: tip-out/product passes owned map.clone() into demoted &GraphVertexI64Map. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-222: graph_bfs_engine missing");
}
