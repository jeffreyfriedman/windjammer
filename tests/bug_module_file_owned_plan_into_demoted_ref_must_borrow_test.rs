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

//! owned `GraphSqlQueryPlan` / `.clone()` into demoted `&GraphSqlQueryPlan` must borrow.
#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str =
    include_str!("fixtures/library_multipass/owned_plan_into_demoted_ref_must_borrow.wj");

#[test]
fn codegen_owned_plan_into_demoted_ref_must_borrow() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let demoted = rs.contains("fn execute(plan: &GraphSqlQueryPlan")
        || rs.contains("execute(plan: &GraphSqlQueryPlan");
    let caller_already_ref = rs.contains("fn execute_logical(plan: &GraphSqlQueryPlan");
    // Both formals `&Plan` → `execute(plan)` is the right Rust. Prefix `&` only when
    // the caller binding is still owned.
    if demoted && !caller_already_ref {
        assert!(
            rs.contains("execute(&plan") || rs.contains("execute(&plan.clone()"),
            "demoted &Plan formal must borrow an owned caller binding. Generated:\n{rs}"
        );
    }
    if demoted {
        assert!(
            !rs.contains("execute(plan.clone()"),
            "no owned clone into &. Generated:\n{rs}"
        );
    }
    assert!(ok, "fixture must cargo-check. Generated:\n{rs}");
}
