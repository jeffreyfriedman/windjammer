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

//! WDB-257: `vertices` into `graph_pagerank_init_scores` must match formal ownership.
//!
//! Tip historically demoted `&mut Vec` while emitting `&mut vertices.clone()`. Tip now
//! keeps owned `vertices: Vec<i64>` + `vertices.clone()` — correct. Gate accepts
//! owned+clone or demoted+`&mut vertices` (no clone).

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
    let mut owned = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("pagerank");
        if text.contains("fn graph_pagerank_init_scores(vertices: &mut Vec<i64>") {
            demoted = true;
            break;
        }
        if text.contains("fn graph_pagerank_init_scores(vertices: Vec<i64>") {
            owned = true;
            break;
        }
    }
    assert!(
        demoted || owned,
        "WDB-257: graph_pagerank_init_scores Vec formal missing (owned or &mut)"
    );

    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("pagerank");
        let bad = if demoted {
            text.contains("graph_pagerank_init_scores(&mut vertices.clone())")
        } else {
            text.contains("graph_pagerank_init_scores(&mut vertices")
                && !text.contains("graph_pagerank_init_scores(vertices.clone())")
                && !text.contains("graph_pagerank_init_scores(vertices)")
        };
        eprintln!(
            "WDB-257 demoted={} owned={} bad={} path={}",
            demoted,
            owned,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-257 RED: tip-out/product call-site ownership mismatches init_scores formal. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-257: graph_pagerank_engine missing");
}
