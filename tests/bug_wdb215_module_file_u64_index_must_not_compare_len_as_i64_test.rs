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

//! WDB-215: `u64` loop index must not compare against `len() as i64`.
//!
//! Product residual (~22× in lsqb_query_engine; census u64←i64 dominant):
//!   `let mut pi = 0_u64; while pi < ((persons.len() as i64))`
//! → expected `u64`, found `i64`. Cast must be `as u64` (or index typed i64).

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str =
    include_str!("fixtures/library_multipass/wdb215_u64_index_len_compare.wj");

#[test]
fn wdb215_codegen_u64_index_must_not_compare_len_as_i64() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let bad = rs.contains("len() as i64)") && rs.contains("while pi <");
    assert!(
        !bad,
        "WDB-215: u64 index vs len must not cast len to i64. Generated:\n{rs}"
    );
    assert!(ok, "WDB-215 fixture must cargo-check. Generated:\n{rs}");
}

use std::path::PathBuf;

#[test]
fn wdb215_tip_out_lsqb_must_not_compare_u64_index_to_len_as_i64() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("lsqb_query_engine.rs"),
        gen.join("graph/lsqb_query_engine.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("lsqb");
        let has_u64_idx = text.contains("0_u64") && text.contains("while pi <");
        let bad = text.contains("len() as i64)")
            && (text.contains("while pi <")
                || text.contains("while i2 <")
                || text.contains("while i3 <")
                || text.contains("while pk <")
                || text.contains("while tag_index <"));
        eprintln!(
            "WDB-215 has_u64_idx={} bad={} path={}",
            has_u64_idx,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-215 RED: tip-out/product compares u64 index to len() as i64. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-215: lsqb_query_engine missing");
}
