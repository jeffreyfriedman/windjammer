//! FAILING REPRO — reusing `db::Connection` across helpers must borrow, not `.clone()`.
//!
//! Ecosystem `wj-migrate` `db_apply::apply_conn` originally split schema/query/apply
//! into helpers taking `conn: db::Connection`. Multipass emitted `conn.clone()` when
//! passing `conn` to a second helper, but `windjammer_runtime::db::Connection` has no
//! `Clone` impl (E0599). Workaround: inline all `conn.execute` / `conn.query` in one fn.

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
fn db_connection_reused_across_helpers_must_borrow_not_clone() {
    let generated = test_utils::assert_stdlib_runtime_links(
        r#"
use std::db

fn ensure_schema(conn: db::Connection) -> Result<(), string> {
    let params = Vec::new()
    match conn.execute("CREATE TABLE IF NOT EXISTS t (id INTEGER)", params) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

fn count_rows(conn: db::Connection) -> Result<int, string> {
    let params = Vec::new()
    match conn.query("SELECT COUNT(*) FROM t", params) {
        Ok(rows) => Ok(rows.len() as int),
        Err(e) => Err(e),
    }
}

pub fn setup_and_count(url: string) -> Result<int, string> {
    match db.connect(url) {
        Ok(conn) => {
            match ensure_schema(conn) {
                Ok(_) => {},
                Err(e) => return Err(e),
            }
            count_rows(conn)
        },
        Err(e) => Err(e),
    }
}
"#,
        &["db::connect", "execute(", "query("],
    );

    assert!(
        !generated.contains("conn.clone()"),
        "Connection reuse across helpers must borrow &, not clone(); emitted:\n{generated}"
    );
}
