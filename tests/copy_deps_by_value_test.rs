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

//! Regression: @derive(Copy) composition deps must pass by value, not &mut AppDeps.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn copy_app_deps_passed_by_value_through_handlers_and_routes() {
    let mut test = MultiFileTest::new();
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
        "adapters/env_account_reader.wj",
        r#"
use ports::readers::AccountReader
use domain::account::Account
use adapters::seed_account_reader::SeedAccountReader

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
        "adapters/seed_account_reader.wj",
        r#"
use ports::readers::AccountReader
use domain::account::Account

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
        "composition/deps.wj",
        r#"
use adapters::env_account_reader::EnvAccountReader

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
        "adapters/routes.wj",
        r#"
use composition::deps::AppDeps
use composition::handlers::fetch_accounts

pub fn match_route(deps: AppDeps, tenant_slug: string) -> int {
    fetch_accounts(deps, tenant_slug).len()
}
"#,
    );
    test.add_file(
        "tests/routes_test.wj",
        r#"
use std::test
use composition::deps::default_deps
use adapters::routes::match_route

@test
fn route_uses_copy_deps_by_value() {
    let deps = default_deps()
    assert_eq(match_route(deps, "demo"), 0)
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
    pub mod routes
}
pub mod composition {
    pub mod deps
    pub mod handlers
}
pub mod tests {
    pub mod routes_test
}
"#,
    );

    test.assert_compiles_without_error();

    let map = test.compile().expect("compile map");
    let routes = map.get("adapters/routes.rs").expect("adapters/routes.rs");
    assert!(
        !routes.contains("&mut AppDeps"),
        "Copy AppDeps must not become &mut at function boundary. Got:\n{routes}"
    );
    let handlers = map
        .get("composition/handlers.rs")
        .expect("composition/handlers.rs");
    assert!(
        !handlers.contains("&mut AppDeps"),
        "Copy AppDeps handlers must pass by value. Got:\n{handlers}"
    );
}

#[test]
fn copy_deps_formal_stays_owned_when_body_passes_to_mutating_callee() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "ports/writer.wj",
        r#"
trait Writer {
    fn write(self, key: string, value: string) -> Result<bool, string>
}
"#,
    );
    test.add_file(
        "adapters/seed_writer.wj",
        r#"
use ports::writer::Writer

pub struct SeedWriter {}

impl Writer for SeedWriter {
    fn write(self, key: string, value: string) -> Result<bool, string> {
        Ok(true)
    }
}
"#,
    );
    test.add_file(
        "composition/deps.wj",
        r#"
use adapters::seed_writer::SeedWriter

pub struct AppDeps {
    pub writer: SeedWriter,
}

pub fn default_deps() -> AppDeps {
    AppDeps { writer: SeedWriter {} }
}
"#,
    );
    test.add_file(
        "composition/post.wj",
        r#"
use ports::writer::Writer
use composition::deps::AppDeps

pub fn post_value(deps: AppDeps, key: string, value: string) -> Result<bool, string> {
    deps.writer.write(key, value)
}
"#,
    );
    test.add_file(
        "composition/handlers.wj",
        r#"
use composition::deps::AppDeps
use composition::post::post_value

pub fn create_entry(deps: AppDeps, key: string, value: string) -> Result<bool, string> {
    post_value(deps, key, value)
}
"#,
    );
    test.add_file(
        "mod.wj",
        r#"
pub mod ports {
    pub mod writer
}
pub mod adapters {
    pub mod seed_writer
}
pub mod composition {
    pub mod deps
    pub mod post
    pub mod handlers
}
"#,
    );

    test.assert_compiles_without_error();

    let map = test.compile().expect("compile map");
    let handlers = map
        .get("composition/handlers.rs")
        .expect("composition/handlers.rs");
    assert!(
        handlers.contains("fn create_entry(deps: AppDeps,")
            || handlers.contains("fn create_entry(deps: AppDeps ,"),
        "Copy AppDeps formal must stay owned when body forwards to mutating callee. Got:\n{handlers}"
    );
    assert!(
        !handlers.contains("fn create_entry(deps: &mut AppDeps"),
        "Copy AppDeps must not become &mut in handler wrapper. Got:\n{handlers}"
    );
}

#[test]
fn copy_deps_routing_passes_owned_not_mut_ref_to_mutating_handler() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "ports/writer.wj",
        r#"
trait Writer {
    fn write(self, key: string, value: string) -> Result<bool, string>
}
"#,
    );
    test.add_file(
        "adapters/seed_writer.wj",
        r#"
use ports::writer::Writer

pub struct SeedWriter {}

impl Writer for SeedWriter {
    fn write(self, key: string, value: string) -> Result<bool, string> {
        Ok(true)
    }
}
"#,
    );
    test.add_file(
        "composition/deps.wj",
        r#"
use adapters::seed_writer::SeedWriter

pub struct AppDeps {
    pub writer: SeedWriter,
}
"#,
    );
    test.add_file(
        "composition/post.wj",
        r#"
use ports::writer::Writer
use composition::deps::AppDeps

pub fn post_value(deps: AppDeps, key: string, value: string) -> Result<bool, string> {
    deps.writer.write(key, value)
}
"#,
    );
    test.add_file(
        "composition/handlers.wj",
        r#"
use composition::deps::AppDeps
use composition::post::post_value

pub fn create_entry(deps: AppDeps, key: string, value: string) -> Result<bool, string> {
    post_value(deps, key, value)
}
"#,
    );
    test.add_file(
        "adapters/routes.wj",
        r#"
use composition::deps::AppDeps
use composition::handlers::create_entry

pub fn match_route(deps: AppDeps, key: string, value: string) -> Result<bool, string> {
    create_entry(deps, key, value)
}
"#,
    );
    test.add_file(
        "mod.wj",
        r#"
pub mod ports { pub mod writer }
pub mod adapters { pub mod seed_writer; pub mod routes }
pub mod composition { pub mod deps; pub mod post; pub mod handlers }
"#,
    );

    test.assert_compiles_without_error();
    let map = test.compile().expect("compile map");
    let routes = map.get("adapters/routes.rs").expect("adapters/routes.rs");
    assert!(
        routes.contains("create_entry(deps,") || routes.contains("create_entry( deps,"),
        "Copy deps must pass by value to mutating handler. Got:\n{routes}"
    );
    assert!(
        !routes.contains("create_entry(&mut deps"),
        "must not pass &mut for Copy AppDeps. Got:\n{routes}"
    );
}

#[test]
fn copy_deps_call_site_owned_when_callee_has_stale_mut_borrowed_metadata() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "ports/writer.wj",
        r#"
trait Writer {
    fn write(self, key: string, value: string) -> Result<bool, string>
}
"#,
    );
    test.add_file(
        "adapters/seed_writer.wj",
        r#"
use ports::writer::Writer

pub struct SeedWriter {}

impl Writer for SeedWriter {
    fn write(self, key: string, value: string) -> Result<bool, string> {
        Ok(true)
    }
}
"#,
    );
    test.add_file(
        "composition/deps.wj",
        r#"
use adapters::seed_writer::SeedWriter

pub struct AppDeps {
    pub writer: SeedWriter,
}
"#,
    );
    test.add_file(
        "composition/post.wj",
        r#"
use ports::writer::Writer
use composition::deps::AppDeps

pub fn post_value(deps: AppDeps, key: string, value: string) -> Result<bool, string> {
    deps.writer.write(key, value)
}
"#,
    );
    test.add_file(
        "composition/handlers.wj",
        r#"
use composition::deps::AppDeps
use composition::post::post_value

pub fn create_entry(deps: AppDeps, key: string, value: string) -> Result<bool, string> {
    let _ = deps.writer
    post_value(deps, key, value)
}
"#,
    );
    test.add_file(
        "adapters/routes.wj",
        r#"
use composition::deps::AppDeps
use composition::handlers::create_entry

pub fn match_route(deps: AppDeps, key: string, value: string) -> Result<bool, string> {
    create_entry(deps, key, value)
}
"#,
    );
    test.add_file(
        "mod.wj",
        r#"
pub mod ports { pub mod writer }
pub mod adapters { pub mod seed_writer; pub mod routes }
pub mod composition { pub mod deps; pub mod post; pub mod handlers }
"#,
    );

    test.assert_compiles_without_error();
    let map = test.compile().expect("compile map");
    let routes = map.get("adapters/routes.rs").expect("adapters/routes.rs");
    assert!(
        routes.contains("create_entry(deps,") || routes.contains("create_entry( deps,"),
        "Copy deps must pass by value even when callee body mutates fields. Got:\n{routes}"
    );
    assert!(
        !routes.contains("create_entry(&mut deps"),
        "must not pass &mut for Copy AppDeps with stale MutBorrowed metadata. Got:\n{routes}"
    );
}
