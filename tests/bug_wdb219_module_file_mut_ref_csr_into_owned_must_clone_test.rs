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

//! WDB-219: demoted `&mut DenseCsr` into owned `DenseCsr` formal must clone/move.
//!
//! Product residual after tip regen of graph_sql_query_port:
//!   `execute(..., csr: DenseCsr)` called with `&mut csr` / `&mut filtered`
//!   → expected `DenseCsr`, found `&mut DenseCsr`.
//! Inverse of WDB-217. Signature-driven.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str =
    include_str!("fixtures/library_multipass/wdb219_mut_ref_csr_into_owned_must_clone.wj");

#[test]
fn wdb219_codegen_owned_csr_formal_must_receive_owned() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let owned = rs.contains("fn execute(csr: DenseCsr") || rs.contains("execute(csr: DenseCsr");
    let bad = owned && rs.contains("execute(&mut csr)") && !rs.contains("execute(csr.clone())");
    assert!(
        !bad,
        "WDB-219: owned DenseCsr formal must not receive &mut. Generated:\n{rs}"
    );
    assert!(ok, "WDB-219 fixture must cargo-check. Generated:\n{rs}");
}

use std::path::PathBuf;

#[test]
fn wdb219_tip_out_graph_sql_must_not_pass_mut_ref_into_owned_csr() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_sql_query_port.rs"),
        gen.join("graph/graph_sql_query_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("graph_sql");
        let owned_formal = text.contains("fn execute(&self, plan: &GraphSqlQueryPlan, csr: DenseCsr")
            || text.contains("plan: &GraphSqlQueryPlan, csr: DenseCsr)");
        let call_mut = text.contains(", &mut csr)") || text.contains(", &mut filtered)");
        let bad = owned_formal && call_mut;
        eprintln!(
            "WDB-219 owned_formal={} bad={} path={}",
            owned_formal,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-219 RED: tip-out/product passes &mut DenseCsr into owned csr formal. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-219: graph_sql_query_port missing");
}
