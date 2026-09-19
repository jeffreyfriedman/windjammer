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

//! WDB-311: residual `u32 = 0_usize` outside wave1 CLI (timeseries + graph_vertex_map).
//!
//! Twin of WDB-298/308 for remaining tip-out/gen sites after adjacency/CDLP sync:
//!   `let mut i: u32 = 0_usize;` (+ nested `j`) → E0308.
//! Prefer `0_u32` (or `0` with u32 peer).

use std::path::PathBuf;

#[test]
fn wdb311_tip_out_timeseries_vertex_map_must_not_emit_u32_eq_0_usize() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("timeseries_ingest_port.rs"),
        tip.join("timeseries/timeseries_ingest_port.rs"),
        tip.join("graph_vertex_map.rs"),
        tip.join("graph/graph_vertex_map.rs"),
        gen.join("timeseries/timeseries_ingest_port.rs"),
        gen.join("graph/graph_vertex_map.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("port");
        if text.contains("u32 = 0_usize") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-311: tip-out/gen timeseries/vertex_map missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-311 RED: tip-out/product still has u32=0_usize in:\n  {}",
        bad_paths.join("\n  ")
    );
}
