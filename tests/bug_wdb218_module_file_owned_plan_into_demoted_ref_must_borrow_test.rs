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

//! WDB-218: owned `GraphSqlQueryPlan` / `.clone()` into demoted `&GraphSqlQueryPlan` must borrow.
//!
//! Product residual (~5×), tip-out/gen graph_sql_query_port:
//!   `execute(&self, plan: &GraphSqlQueryPlan, …)` called with
//!   `self.execute(graph_sql_logical_to_query(…).clone(), …)` → expected `&`, found owned.
//! Twin of WDB-203/212. Signature-driven.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str =
    include_str!("fixtures/library_multipass/wdb218_owned_plan_into_demoted_ref_must_borrow.wj");

#[test]
fn wdb218_codegen_owned_plan_into_demoted_ref_must_borrow() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let demoted = rs.contains("fn execute(plan: &GraphSqlQueryPlan")
        || rs.contains("execute(plan: &GraphSqlQueryPlan");
    let bad = demoted
        && (rs.contains("execute(plan.clone()") || rs.contains("execute(plan)"))
        && !rs.contains("execute(&plan")
        && !rs.contains("execute(&plan.clone()");
    // If demoted, call must borrow; if owned formal, bare plan is fine.
    if demoted {
        assert!(
            rs.contains("execute(&plan") || rs.contains("execute(&plan.clone()"),
            "WDB-218: demoted &Plan formal must borrow at call site. Generated:\n{rs}"
        );
        assert!(!rs.contains("execute(plan.clone()"), "WDB-218: no owned clone into &. Generated:\n{rs}");
    }
    assert!(ok, "WDB-218 fixture must cargo-check. Generated:\n{rs}");
    let _ = bad;
}

use std::path::PathBuf;

#[test]
fn wdb218_tip_out_graph_sql_must_borrow_owned_plan_into_demoted_execute() {
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
        let demoted = text.contains("plan: &GraphSqlQueryPlan");
        // Owned clone / owned temp into &Plan without leading &
        let bad = demoted
            && (text.contains("self.execute(graph_sql_logical_to_query(")
                || text.contains("host.execute(graph_sql_logical_to_query("))
            && !text.contains("self.execute(&graph_sql_logical_to_query(")
            && !text.contains("host.execute(&graph_sql_logical_to_query(");
        eprintln!(
            "WDB-218 demoted={} bad={} path={}",
            demoted,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-218 RED: tip-out/product passes owned plan into demoted &GraphSqlQueryPlan. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-218: graph_sql_query_port missing");
}
