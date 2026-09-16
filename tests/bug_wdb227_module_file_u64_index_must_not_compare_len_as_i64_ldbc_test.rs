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

//! WDB-227: `u64` loop index must not compare against `len() as i64` (LDBC validation).
//!
//! Twin of WDB-215 (lsqb tip GREEN). Product residual still in tip-out/gen
//! graph_ldbc_validation_engine:
//!   `while i < ((reference.len() as i64))` with `i: u64` → E0308.
//! Signature/width-driven: cast `len()` to `u64` (or keep index i64).

use std::path::PathBuf;

#[test]
fn wdb227_tip_out_ldbc_must_not_compare_u64_index_to_len_as_i64() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_ldbc_validation_engine.rs"),
        gen.join("graph/graph_ldbc_validation_engine.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("ldbc");
        let bad = text.contains("while i < ((reference.len() as i64))")
            || text.contains("while i < ((csr_vertices.len() as i64))");
        eprintln!("WDB-227 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-227 RED: tip-out/product compares u64 index to len() as i64 (LDBC). {}",
            path.display()
        );
    }
    assert!(saw, "WDB-227: graph_ldbc_validation_engine missing");
}
