#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! WDB-103: inverse of WDB-099 — **owned** struct formals must not receive `&arg`
//! at call sites (Phase 148 `graph_sql_physical_exec_with_batch(&host2, …)` dogfood).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn wdb103_owned_host_formal_must_move_not_borrow() {
    let source = r#"
pub struct QueryHost {
    pub visits: u64,
}

pub fn exec_with_host(host: QueryHost) -> u64 {
    host.visits
}

pub fn run_query() -> u64 {
    let host = QueryHost { visits: 3 }
    exec_with_host(host)
}
"#;

    let rs = test_utils::compile_single(source);
    assert!(
        !rs.contains("exec_with_host(&host)"),
        "WDB-103: owned QueryHost formal must not receive &host. Got:\n{rs}"
    );
    assert!(
        rs.contains("exec_with_host(host)"),
        "WDB-103: expected move of host into owned formal. Got:\n{rs}"
    );
}
