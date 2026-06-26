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

//! Regression: trait calls through composition deps must pass owned `String` at call sites.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn deps_field_trait_methods_use_owned_string_at_call_site() {
    let mut test = MultiFileTest::new();
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
        "ports/readers.wj",
        r#"
use domain::account::Account

trait AccountReader {
    fn list_accounts(self, tenant_slug: string) -> Vec<Account>
}
"#,
    );
    test.add_file(
        "adapters/seed_account_reader.wj",
        r#"
use ports::readers::AccountReader
use domain::account::Account

@derive(Copy)
pub struct SeedAccountReader {}

impl AccountReader for SeedAccountReader {
    fn list_accounts(self, tenant_slug: string) -> Vec<Account> {
        let _ = tenant_slug
        vec![]
    }
}
"#,
    );
    test.add_file(
        "adapters/env_account_reader.wj",
        r#"
use ports::readers::AccountReader
use domain::account::Account
use adapters::seed_account_reader::SeedAccountReader

@derive(Copy)
pub struct EnvAccountReader {}

impl AccountReader for EnvAccountReader {
    fn list_accounts(self, tenant_slug: string) -> Vec<Account> {
        let reader = SeedAccountReader {}
        reader.list_accounts(tenant_slug)
    }
}
"#,
    );
    test.add_file(
        "composition/deps.wj",
        r#"
use adapters::env_account_reader::EnvAccountReader

@derive(Copy)
pub struct AppDeps {
    pub account_reader: EnvAccountReader,
}

pub fn default_deps() -> AppDeps {
    AppDeps { account_reader: EnvAccountReader {} }
}
"#,
    );
    test.add_file(
        "composition/handlers.wj",
        r#"
use domain::account::Account
use ports::readers::AccountReader
use composition::deps::AppDeps

pub fn fetch_accounts(deps: AppDeps, tenant_slug: string) -> Vec<Account> {
    deps.account_reader.list_accounts(tenant_slug)
}
"#,
    );
    test.add_file(
        "tests/handlers_test.wj",
        r#"
use std::test
use composition::deps::default_deps
use composition::handlers::fetch_accounts

@test
fn fetch_accounts_via_deps_passes_owned_slug() {
    let deps = default_deps()
    assert_eq(fetch_accounts(deps, "demo").len(), 0)
}
"#,
    );
    test.add_file(
        "mod.wj",
        r#"
pub mod domain {
    pub mod account
}
pub mod ports {
    pub mod readers
}
pub mod adapters {
    pub mod seed_account_reader
    pub mod env_account_reader
}
pub mod composition {
    pub mod deps
    pub mod handlers
}
pub mod tests {
    pub mod handlers_test
}
"#,
    );

    test.assert_compiles_without_error();

    let map = test.compile().expect("compile map");
    let rs = map
        .get("composition/handlers.rs")
        .expect("composition/handlers.rs");
    assert!(
        !rs.contains("list_accounts(&"),
        "deps trait call must pass owned String. Got:\n{rs}"
    );
}
