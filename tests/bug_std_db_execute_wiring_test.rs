//! FAILING REPRO — `std::db` connect + execute must be usable for migrations.
//!
//! Ecosystem `wj-migrate` has domain SQL + docker smoke; in-WJ apply needs:
//! `db.connect(url)?` then `conn.execute(sql, params)?`.

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
    feature = "integration_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn std_db_connect_execute_codegen_resolves() {
    let source = r#"
use std::db

pub fn apply(url: string, sql: string) -> Result<int, string> {
    match db.connect(url) {
        Ok(conn) => {
            let mut params = Vec::new()
            conn.execute(sql, params)
        },
        Err(e) => Err(e),
    }
}
"#;
    let (generated, ok) = test_utils::compile_single_check(source);
    assert!(
        ok,
        "std::db.connect/execute must compile, got:\n{generated}"
    );
    let connect_ok = generated.contains("db::connect")
        || generated.contains("windjammer_runtime::db::connect");
    let execute_ok = generated.contains("execute(");
    assert!(
        connect_ok && execute_ok,
        "db.connect + execute must map to runtime, got:\n{generated}"
    );
}
