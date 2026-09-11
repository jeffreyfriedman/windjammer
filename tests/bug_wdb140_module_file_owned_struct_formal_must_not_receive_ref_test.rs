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
))]

//! WDB-140: owned struct formal receives `&plan` (over-borrow) → E0308
//! (`expected GraphSqlLogicalPlan, found &GraphSqlLogicalPlan`).
//!
//! WindjammerDB CQ-C5 (`graph_sql_datafusion_port.rs`):
//!   `graph_sql_logical_to_datafusion_sql_edges_src_in(&logical, &allow_srcs)`
//!   while first formal is owned `GraphSqlLogicalPlan` (second is demoted `&Vec`).
//!
//! Inverse of demoted-`&T` + owned clone (WDB-125/126). Expected: `logical.clone()`
//! (or move) into the owned formal — not `&logical`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod plan
pub mod sql
"#;

const PLAN: &str = r#"
pub struct LogicalPlan {
    pub table_id: u32,
}

pub fn to_sql(plan: LogicalPlan, allow: Vec<i64>) -> u32 {
    let mut n: u32 = plan.table_id
    for _a in allow {
        n = n + 1
    }
    n
}
"#;

const SQL: &str = r#"
use crate::plan::LogicalPlan
use crate::plan::to_sql

/// Cap leaf borrows `allow` → tip demotes `&Vec`; `plan` must stay owned move/clone.
pub fn render(plan: LogicalPlan, allow: Vec<i64>) -> u32 {
    let a = to_sql(plan, allow)
    let b = to_sql(plan, allow)
    a + b
}

pub fn cap() -> u32 {
    render(LogicalPlan { table_id: 7 }, vec![1, 2])
}
"#;

fn wdb140_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("plan.wj", PLAN);
    test.add_file("sql.wj", SQL);
    test
}

#[test]
fn wdb140_module_file_owned_struct_formal_must_not_receive_ref() {
    let test = wdb140_fixture();
    let map = test
        .compile()
        .expect("WDB-140 multipass compile should succeed (codegen may still be wrong)");
    let plan_rs = map.get("plan.rs").expect("plan.rs");
    let sql_rs = map.get("sql.rs").expect("sql.rs");

    let callee_owned = {
        let i = plan_rs.find("fn to_sql").unwrap_or(0);
        let sl = &plan_rs[i..plan_rs.len().min(i + 120)];
        sl.contains("plan: LogicalPlan") && !sl.contains("plan: &LogicalPlan")
    };
    let bad_ref = sql_rs.contains("to_sql(&plan")
        || sql_rs.contains("to_sql(&plan.clone()");
    let good = sql_rs.contains("to_sql(plan.clone()")
        || sql_rs.contains("to_sql(plan,");

    eprintln!("WDB-140 plan.rs:\n{plan_rs}\nsql.rs:\n{sql_rs}");
    eprintln!("callee_owned={callee_owned} bad_ref={bad_ref} good={good}");

    if callee_owned && bad_ref {
        panic!(
            "WDB-140 RED: owned LogicalPlan formal must not receive &plan. \
             Product: graph_sql_logical_to_datafusion_sql_edges_src_in(&logical, …)."
        );
    }

    test.cargo_check().expect(
        "WDB-140: owned struct formal must cargo-check (move/clone, not &).",
    );
}
