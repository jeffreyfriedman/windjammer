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

//! WDB-275: demoted `&Vec<f64>` into owned `graph_arena_return_f64` must clone/move.
//!
//! Twin of WDB-241/261. Tip-out PageRank still emits:
//!   `graph_arena_return_f64(&scores)` / `&next` / `&contrib` / `&inv_deg`
//! while formal is `buf: Vec<f64>` → E0308.
//! Signature-driven: move/`clone()` when formal is owned Vec.

use std::path::PathBuf;

#[test]
fn wdb275_tip_out_pagerank_must_clone_ref_vec_into_owned_arena_return_f64() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let port_paths = [
        tip.join("graph_scratch_arena_port.rs"),
        gen.join("graph/graph_scratch_arena_port.rs"),
    ];
    let mut owned = false;
    for path in &port_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("arena");
        if text.contains("fn graph_arena_return_f64(buf: Vec<f64>") {
            owned = true;
            break;
        }
    }
    assert!(
        owned,
        "WDB-275: owned Vec graph_arena_return_f64 formal missing"
    );

    // Prefer tip-out (gen may already move owned args).
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
        let bad = text.contains("graph_arena_return_f64(&scores)")
            || text.contains("graph_arena_return_f64(&next)")
            || text.contains("graph_arena_return_f64(&contrib)")
            || text.contains("graph_arena_return_f64(&inv_deg)");
        eprintln!("WDB-275 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-275 RED: tip-out/product passes &Vec into owned graph_arena_return_f64. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-275: graph_pagerank_engine missing");
}
