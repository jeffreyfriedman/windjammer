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

//! Regression: DB reader trait methods with owned `string` formals must not get
//! `&demo_tenant().slug.clone()` at call sites when body analysis converged to `&str`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn db_item_and_report_readers_use_owned_string_at_call_site() {
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
use domain::item::Item
use domain::report::ReportLine

trait ItemReader {
    fn list_items(self, tenant_slug: string) -> Vec<Item>
}

trait ReportReader {
    fn report_lines(self, tenant_slug: string) -> Vec<ReportLine>
}
"#,
    );
    test.add_file(
        "domain/item.wj",
        r#"
pub struct Item {
    pub code: string,
    pub name: string,
    pub item_type: string,
    pub amount_cents: int,
}
"#,
    );
    test.add_file(
        "domain/report.wj",
        r#"
pub struct ReportLine {
    pub code: string,
    pub name: string,
    pub debit_cents: int,
    pub credit_cents: int,
}
"#,
    );
    test.add_file(
        "adapters/db_session.wj",
        r#"
pub struct Row {}

pub fn query_tenant_rows(sql: string, tenant_slug: string) -> Vec<Row> {
    vec![]
}
"#,
    );
    test.add_file(
        "adapters/db_queries.wj",
        r#"
pub fn list_items_sql() -> string {
    "SELECT 1"
}

pub fn report_lines_sql() -> string {
    "SELECT 2"
}
"#,
    );
    test.add_file(
        "adapters/db_item_reader.wj",
        r#"
use ports::readers::ItemReader
use domain::item::Item
use adapters::db_queries::list_items_sql
use adapters::db_session::query_tenant_rows

pub struct DbItemReader {}

impl ItemReader for DbItemReader {
    fn list_items(self, tenant_slug: string) -> Vec<Item> {
        let mut items = vec![]
        for row in query_tenant_rows(list_items_sql(), tenant_slug) {
            let _ = row
        }
        items
    }
}
"#,
    );
    test.add_file(
        "adapters/db_report_reader.wj",
        r#"
use ports::readers::ReportReader
use domain::report::ReportLine
use adapters::db_queries::report_lines_sql
use adapters::db_session::query_tenant_rows

pub struct DbReportReader {}

impl ReportReader for DbReportReader {
    fn report_lines(self, tenant_slug: string) -> Vec<ReportLine> {
        let mut lines = vec![]
        for row in query_tenant_rows(report_lines_sql(), tenant_slug) {
            let _ = row
        }
        lines
    }
}
"#,
    );
    test.add_file(
        "tests/db_stub_test.wj",
        r#"
use std::test
use domain::tenant::demo_tenant
use adapters::db_item_reader::DbItemReader
use adapters::db_report_reader::DbReportReader
use ports::readers::{ItemReader, ReportReader}

@test
fn db_item_reader_without_database_url_returns_empty() {
    let reader = DbItemReader {}
    assert_eq(reader.list_items(demo_tenant().slug).len(), 0)
}

@test
fn db_report_reader_without_database_url_returns_empty() {
    let reader = DbReportReader {}
    assert_eq(reader.report_lines(demo_tenant().slug).len(), 0)
}
"#,
    );
    test.add_file(
        "mod.wj",
        r#"
pub mod domain {
    pub mod tenant
    pub mod item
    pub mod report
}
pub mod ports {
    pub mod readers
}
pub mod adapters {
    pub mod db_session
    pub mod db_queries
    pub mod db_item_reader
    pub mod db_report_reader
}
pub mod tests {
    pub mod db_stub_test
}
"#,
    );

    test.assert_compiles_without_error();

    let map = test.compile().expect("compile map");
    let rs = map
        .get("tests/db_stub_test.rs")
        .expect("tests/db_stub_test.rs");
    assert!(
        !rs.contains("report_lines(&"),
        "owned String trait param must not get borrow prefix. Got:\n{rs}"
    );
    assert!(
        !rs.contains("list_items(&"),
        "owned String trait param must not get borrow prefix. Got:\n{rs}"
    );
}
