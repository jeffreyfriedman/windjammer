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

//! WDB-231: `usize` loop index must not compound-add `1 as i32`.
//!
//! Product residual (~5× usize←i32), tip-out/gen graph_batch_engine:
//!   `i += 1 as i32` while `i: usize` → E0308 expected usize, found i32.
//! Inverse of P3.304 (i32 += 1 as usize). Prefer `i += 1` matching index width.

use std::path::PathBuf;

#[test]
fn wdb231_tip_out_batch_must_not_add_i32_to_usize_index() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_batch_engine.rs"),
        gen.join("graph/graph_batch_engine.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("batch");
        let bad = text.contains("i += 1 as i32") || text.contains("i += 1_i32");
        eprintln!("WDB-231 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-231 RED: tip-out/product adds i32 literal to usize index. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-231: graph_batch_engine missing");
}
