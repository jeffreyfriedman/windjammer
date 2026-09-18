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

//! WDB-277: demoted `&Vec` into owned `graph_par_cdlp_bind` must clone.
//!
//! Twin of WDB-276 (par_bfs_bind). Tip-out CDLP parallel still emits:
//!   `graph_par_cdlp_bind(&out.offsets, &out.neighbors, &labels, &new_labels)`
//! while formals are owned `Vec<u32>` → E0308.

use std::path::PathBuf;

#[test]
fn wdb277_tip_out_cdlp_must_clone_ref_vec_into_owned_par_cdlp_bind() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let port_paths = [
        tip.join("graph_parallel_port.rs"),
        gen.join("graph/graph_parallel_port.rs"),
    ];
    let mut owned = false;
    for path in &port_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("parallel");
        if text.contains("fn graph_par_cdlp_bind(offsets: Vec<u32>") {
            owned = true;
            break;
        }
    }
    assert!(
        owned,
        "WDB-277: owned Vec graph_par_cdlp_bind formal missing"
    );

    let engine_paths = if tip.join("graph_cdlp_engine.rs").exists() {
        vec![tip.join("graph_cdlp_engine.rs")]
    } else {
        vec![gen.join("graph/graph_cdlp_engine.rs")]
    };
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("cdlp");
        let bad = text.contains(
            "graph_par_cdlp_bind(&out.offsets, &out.neighbors, &labels, &new_labels)",
        );
        eprintln!("WDB-277 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-277 RED: tip-out/product passes &Vec into owned graph_par_cdlp_bind. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-277: graph_cdlp_engine missing");
}
