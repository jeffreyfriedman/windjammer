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

//! std::db runtime Rust APIs take `&str`; Windjammer `string` params must auto-borrow at call sites.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn std_db_connect_borrows_owned_url() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "adapter.wj",
        r#"
use std::db

pub fn open_from_url(url: string) -> bool {
    match db.connect(url) {
        Ok(_conn) => true,
        Err(_) => false,
    }
}
"#,
    );
    test.add_file("mod.wj", "pub mod adapter");

    let map = test.compile().expect("compile");
    let rs = map.get("adapter.rs").expect("adapter.rs");
    assert!(
        rs.contains("db::connect(&url") || rs.contains("db::connect( &url"),
        "owned url must borrow for runtime db::connect(&str). Got:\n{rs}"
    );
    test.assert_compiles_without_error();
}

#[test]
fn std_db_query_borrows_owned_sql() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "adapter.wj",
        r#"
use std::db

pub fn run_query(conn: Connection, sql: string, tenant: string) -> int {
    match conn.query(sql, vec![tenant]) {
        Ok(rows) => rows.len(),
        Err(_) => 0,
    }
}
"#,
    );
    test.add_file("mod.wj", "pub mod adapter");

    let map = test.compile().expect("compile");
    let rs = map.get("adapter.rs").expect("adapter.rs");
    assert!(
        rs.contains("query(&sql") || rs.contains("query( &sql"),
        "owned sql must borrow for runtime Connection::query(&str, ...). Got:\n{rs}"
    );
    test.assert_compiles_without_error();
}
