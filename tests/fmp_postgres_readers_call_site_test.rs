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

//! Regression: Postgres reader trait methods with owned `string` formals must not get
//! `&demo_tenant().slug.clone()` at call sites when body analysis converged to `&str`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn postgres_trial_balance_and_account_readers_use_owned_string_at_call_site() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "domain/tenant.wj",
        r#"
pub struct Tenant {
    pub slug: string,
}

pub fn demo_tenant() -> Tenant {
    Tenant { slug: "demo" }
}
"#,
    );
    test.add_file(
        "ports/readers.wj",
        r#"
use domain::account::Account
use domain::trial_balance::TrialBalanceLine

trait AccountReader {
    fn list_accounts(self, tenant_slug: string) -> Vec<Account>
}

trait TrialBalanceReader {
    fn trial_balance_lines(self, tenant_slug: string) -> Vec<TrialBalanceLine>
}
"#,
    );
    test.add_file(
        "domain/account.wj",
        r#"
pub struct Account {
    pub code: string,
    pub name: string,
    pub account_type: string,
    pub balance_cents: int,
}
"#,
    );
    test.add_file(
        "domain/trial_balance.wj",
        r#"
pub struct TrialBalanceLine {
    pub code: string,
    pub name: string,
    pub debit_cents: int,
    pub credit_cents: int,
}
"#,
    );
    test.add_file(
        "adapters/postgres_session.wj",
        r#"
pub struct Row {}

pub fn query_tenant_rows(sql: string, tenant_slug: string) -> Vec<Row> {
    vec![]
}
"#,
    );
    test.add_file(
        "adapters/postgres_queries.wj",
        r#"
pub fn list_accounts_sql() -> string {
    "SELECT 1"
}

pub fn trial_balance_lines_sql() -> string {
    "SELECT 2"
}
"#,
    );
    test.add_file(
        "adapters/postgres_account_reader.wj",
        r#"
use ports::readers::AccountReader
use domain::account::Account
use adapters::postgres_queries::list_accounts_sql
use adapters::postgres_session::query_tenant_rows

@derive(Copy)
pub struct PostgresAccountReader {}

impl AccountReader for PostgresAccountReader {
    fn list_accounts(self, tenant_slug: string) -> Vec<Account> {
        let mut accounts = vec![]
        for row in query_tenant_rows(list_accounts_sql(), tenant_slug) {
            let _ = row
        }
        accounts
    }
}
"#,
    );
    test.add_file(
        "adapters/postgres_trial_balance_reader.wj",
        r#"
use ports::readers::TrialBalanceReader
use domain::trial_balance::TrialBalanceLine
use adapters::postgres_queries::trial_balance_lines_sql
use adapters::postgres_session::query_tenant_rows

@derive(Copy)
pub struct PostgresTrialBalanceReader {}

impl TrialBalanceReader for PostgresTrialBalanceReader {
    fn trial_balance_lines(self, tenant_slug: string) -> Vec<TrialBalanceLine> {
        let mut lines = vec![]
        for row in query_tenant_rows(trial_balance_lines_sql(), tenant_slug) {
            let _ = row
        }
        lines
    }
}
"#,
    );
    test.add_file(
        "tests/postgres_stub_test.wj",
        r#"
use std::test
use domain::tenant::demo_tenant
use adapters::postgres_account_reader::PostgresAccountReader
use adapters::postgres_trial_balance_reader::PostgresTrialBalanceReader
use ports::readers::{AccountReader, TrialBalanceReader}

@test
fn postgres_account_reader_without_database_url_returns_empty() {
    let reader = PostgresAccountReader {}
    assert_eq(reader.list_accounts(demo_tenant().slug).len(), 0)
}

@test
fn postgres_trial_balance_reader_without_database_url_returns_empty() {
    let reader = PostgresTrialBalanceReader {}
    assert_eq(reader.trial_balance_lines(demo_tenant().slug).len(), 0)
}
"#,
    );
    test.add_file(
        "mod.wj",
        r#"
pub mod domain {
    pub mod tenant
    pub mod account
    pub mod trial_balance
}
pub mod ports {
    pub mod readers
}
pub mod adapters {
    pub mod postgres_session
    pub mod postgres_queries
    pub mod postgres_account_reader
    pub mod postgres_trial_balance_reader
}
pub mod tests {
    pub mod postgres_stub_test
}
"#,
    );

    test.assert_compiles_without_error();

    let map = test.compile().expect("compile map");
    let rs = map
        .get("tests/postgres_stub_test.rs")
        .expect("tests/postgres_stub_test.rs");
    assert!(
        !rs.contains("trial_balance_lines(&"),
        "owned String trait param must not get borrow prefix. Got:\n{rs}"
    );
    assert!(
        !rs.contains("list_accounts(&"),
        "owned String trait param must not get borrow prefix. Got:\n{rs}"
    );
}
