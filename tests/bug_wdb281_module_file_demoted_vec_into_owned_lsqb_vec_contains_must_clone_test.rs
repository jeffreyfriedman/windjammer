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

//! WDB-281: demoted `&Vec<i64>` into owned `lsqb_vec_contains` must clone.
//!
//! Twin of WDB-241/276. Tip-out lsqb_typed_graph still emits:
//!   `lsqb_vec_contains(&items, value)` while formal is `items: Vec<i64>` → E0308.
//! Signature-driven: `items.clone()` / move when formal is owned.

use std::path::PathBuf;

#[test]
fn wdb281_tip_out_lsqb_must_clone_ref_vec_into_owned_vec_contains() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("lsqb_typed_graph.rs"),
        gen.join("graph/lsqb_typed_graph.rs"),
    ];
    let mut owned = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("lsqb");
        if text.contains("fn lsqb_vec_contains(items: Vec<i64>") {
            owned = true;
            break;
        }
    }
    assert!(
        owned,
        "WDB-281: owned Vec lsqb_vec_contains formal missing"
    );

    let engine_paths = if tip.join("lsqb_typed_graph.rs").exists() {
        vec![tip.join("lsqb_typed_graph.rs")]
    } else {
        vec![gen.join("graph/lsqb_typed_graph.rs")]
    };
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("lsqb");
        let bad = text.contains("lsqb_vec_contains(&items,")
            && !text.contains("lsqb_vec_contains(items.clone(),")
            && !text.contains("lsqb_vec_contains(items,");
        eprintln!("WDB-281 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-281 RED: tip-out/product passes &Vec into owned lsqb_vec_contains. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-281: lsqb_typed_graph missing");
}
