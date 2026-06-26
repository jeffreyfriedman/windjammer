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

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn std_db_platform_shape_compiles() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "postgres_account_reader.wj",
        r#"
use std::db

pub struct Account {
    code: string,
    name: string,
    account_type: string,
    balance_cents: int,
}

pub fn database_url() -> Option<string> {
    None
}

pub fn list_accounts(tenant_slug: string) -> Vec<Account> {
    let url = match database_url() {
        Some(value) if value.len() > 0 => value,
        _ => return vec![],
    }

    let conn = match db.connect(url) {
        Ok(c) => c,
        Err(_) => return vec![],
    }

    let sql = "
        SELECT a.code FROM accounts a WHERE t.slug = $1
    "

    let rows = match conn.query(sql, vec![tenant_slug]) {
        Ok(r) => r,
        Err(_) => return vec![],
    }

    let mut accounts = vec![]
    for row in rows {
        let code = match row.get_string("code") {
            Ok(v) => v,
            Err(_) => continue,
        }
        accounts.push(Account {
            code: code,
            name: "n",
            account_type: "Asset",
            balance_cents: 0,
        })
    }
    accounts
}
"#,
    );
    test.add_file("mod.wj", "pub mod postgres_account_reader");

    let map = test.compile().expect("compile");
    let rs = map.get("postgres_account_reader.rs").expect("rs");
    assert!(
        rs.contains("db::connect(&url") || rs.contains("db::connect(url"),
        "connect must accept owned url. Got:\n{rs}"
    );
    assert!(
        rs.contains("query(&sql") || rs.contains("query(sql"),
        "query must accept owned sql. Got:\n{rs}"
    );
    test.assert_compiles_without_error();
}
