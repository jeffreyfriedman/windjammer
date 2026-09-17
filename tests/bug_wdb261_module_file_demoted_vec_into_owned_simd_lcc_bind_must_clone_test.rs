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

//! WDB-261: demoted `&Vec` into owned `graph_simd_lcc_bind_oriented` must clone (LCC).
//!
//! Twin of WDB-241/258/259. Product residual tip-out/gen graph_lcc_engine:
//!   `graph_simd_lcc_bind_oriented(neighbors: Vec<u32>, offsets: Vec<u32>, tri: Vec<u32>)`
//!   called as `…(self.fwd.neighbors.clone(), &self.fwd.offsets, &self.tri)`
//! → E0308 expected `Vec`, found `&Vec` for offsets/tri.
//! Signature-driven: clone offsets/tri (or demote formals to `&Vec`).

use std::path::PathBuf;

#[test]
fn wdb261_tip_out_lcc_must_clone_ref_vec_into_owned_simd_bind() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let simd_paths = [
        tip.join("graph_simd_port.rs"),
        gen.join("graph/graph_simd_port.rs"),
    ];
    let mut owned = false;
    for path in &simd_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("simd");
        if text.contains(
            "fn graph_simd_lcc_bind_oriented(neighbors: Vec<u32>, offsets: Vec<u32>, tri: Vec<u32>)",
        ) {
            owned = true;
            break;
        }
    }
    assert!(
        owned,
        "WDB-261: owned Vec formals for graph_simd_lcc_bind_oriented missing"
    );

    let engine_paths = [
        tip.join("graph_lcc_engine.rs"),
        gen.join("graph/graph_lcc_engine.rs"),
    ];
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("lcc");
        let bad = text.contains(
            "graph_simd_lcc_bind_oriented(self.fwd.neighbors.clone(), &self.fwd.offsets, &self.tri)",
        );
        eprintln!("WDB-261 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-261 RED: tip-out/product passes &Vec offsets/tri into owned simd bind. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-261: graph_lcc_engine missing");
}
