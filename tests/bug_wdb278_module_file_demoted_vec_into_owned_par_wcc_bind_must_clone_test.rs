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

//! WDB-278: demoted `&Vec` into owned `graph_par_wcc_bind` must clone.
//!
//! Twin of WDB-276/277. Tip-out WCC afforest still emits:
//!   `graph_par_wcc_bind(&out.offsets, &out.neighbors)`
//! while formals are owned `Vec<u32>` → E0308.

use std::path::PathBuf;

#[test]
fn wdb278_tip_out_wcc_must_clone_ref_vec_into_owned_par_wcc_bind() {
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
        if text.contains("fn graph_par_wcc_bind(offsets: Vec<u32>") {
            owned = true;
            break;
        }
    }
    assert!(
        owned,
        "WDB-278: owned Vec graph_par_wcc_bind formal missing"
    );

    let engine_paths = if tip.join("graph_wcc_engine.rs").exists() {
        vec![tip.join("graph_wcc_engine.rs")]
    } else {
        vec![gen.join("graph/graph_wcc_engine.rs")]
    };
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("wcc");
        let bad = text.contains("graph_par_wcc_bind(&out.offsets, &out.neighbors)");
        eprintln!("WDB-278 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-278 RED: tip-out/product passes &Vec into owned graph_par_wcc_bind. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-278: graph_wcc_engine missing");
}
