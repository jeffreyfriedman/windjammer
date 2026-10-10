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

//! DB-219: demoted `&mut DenseCsr` into owned `DenseCsr` formal must clone/move.
//!
//! Product residual after tip regen of graph_sql_query_port:
//!   `execute(..., csr: DenseCsr)` called with `&mut csr` / `&mut filtered`
//!   → expected `DenseCsr`, found `&mut DenseCsr`.
//! Inverse of DB-217. Signature-driven.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str =
    include_str!("fixtures/library_multipass/mut_ref_csr_into_owned_must_clone.wj");

#[test]
fn codegen_owned_csr_formal_must_receive_owned() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let owned = rs.contains("fn execute(csr: DenseCsr") || rs.contains("execute(csr: DenseCsr");
    let bad = owned && rs.contains("execute(&mut csr)") && !rs.contains("execute(csr.clone())");
    assert!(
        !bad,
        "owned DenseCsr formal must not receive &mut. Generated:\n{rs}"
    );
    assert!(ok, "fixture must cargo-check. Generated:\n{rs}");
}
