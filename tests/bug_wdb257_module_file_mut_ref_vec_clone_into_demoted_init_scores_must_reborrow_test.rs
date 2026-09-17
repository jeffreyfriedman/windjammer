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

//! WDB-257: `&mut vertices.clone()` into demoted `&mut Vec<i64>` init_scores must reborrow.
//!
//! Product residual tip-out/gen graph_pagerank_engine:
//!   `graph_pagerank_init_scores(vertices: &mut Vec<i64>)`
//!   called as `graph_pagerank_init_scores(&mut vertices.clone())`
//! → temporary mut-ref to clone (E0716 / wrong ownership), expected `&mut vertices`.
//! Signature-driven: pass `&mut vertices` (no clone).

use std::path::PathBuf;

#[test]
fn wdb257_tip_out_pagerank_must_reborrow_mut_vec_into_demoted_init_scores() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let engine_paths = [
        tip.join("graph_pagerank_engine.rs"),
        gen.join("graph/graph_pagerank_engine.rs"),
    ];
    let mut demoted = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("pagerank");
        if text.contains("fn graph_pagerank_init_scores(vertices: &mut Vec<i64>") {
            demoted = true;
            break;
        }
    }
    assert!(
        demoted,
        "WDB-257: demoted &mut Vec graph_pagerank_init_scores formal missing"
    );

    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("pagerank");
        let bad = text.contains("graph_pagerank_init_scores(&mut vertices.clone())");
        eprintln!("WDB-257 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-257 RED: tip-out/product passes &mut vertices.clone() into demoted &mut Vec init_scores. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-257: graph_pagerank_engine missing");
}
