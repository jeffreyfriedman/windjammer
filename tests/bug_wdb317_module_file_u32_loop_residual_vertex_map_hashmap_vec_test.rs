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

//! WDB-317: residual `u32 = 0_usize` in graph_vertex_map.hashmap / .vec variants.
//!
//! Twin of WDB-311 (main `graph_vertex_map.rs`) for alternate backend files still RED:
//!   `let mut i: u32 = 0_usize;` (+ nested `j`) → E0308.
//! Prefer `0_u32`.

use std::path::PathBuf;

#[test]
fn wdb317_tip_out_vertex_map_hashmap_vec_must_not_emit_u32_eq_0_usize() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_vertex_map.hashmap.rs"),
        tip.join("graph_vertex_map.vec.rs"),
        tip.join("graph/graph_vertex_map.hashmap.rs"),
        tip.join("graph/graph_vertex_map.vec.rs"),
        gen.join("graph/graph_vertex_map.hashmap.rs"),
        gen.join("graph/graph_vertex_map.vec.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("vertex_map");
        if text.contains("u32 = 0_usize") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-317: tip-out/gen vertex_map.hashmap/.vec missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-317 RED: tip-out/product still has u32=0_usize in:\n  {}",
        bad_paths.join("\n  ")
    );
}
