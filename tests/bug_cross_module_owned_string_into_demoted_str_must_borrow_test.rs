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
    feature = "codegen_tests",
))]

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const TABLES: &str = r#"
pub fn general_ledger_table_html(account_code: string, as_of: string) -> string {
    "${account_code}:${as_of}"
}
"#;

const READ: &str = r#"
use super::tables::general_ledger_table_html

pub struct Report {
    pub account_code: string,
    pub as_of: string,
}

pub fn render(report: Report) -> string {
    let account_code = report.account_code
    let as_of = report.as_of
    general_ledger_table_html(account_code, as_of)
}
"#;

const MOD: &str = r#"
mod tables;
mod read;
"#;

#[test]
fn cross_module_owned_string_into_demoted_str_must_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("tables.wj", TABLES);
    test.add_file("read.wj", READ);
    let map = test.compile().expect("cross-module compile");
    let rs = map
        .get("read.rs")
        .or_else(|| map.get("lib.rs"))
        .expect("read.rs");
    eprintln!("cross-module read.rs:\n{rs}");
    let call = rs
        .lines()
        .find(|l| l.contains("general_ledger_table_html("))
        .unwrap_or(rs);
    assert!(
        call.contains("&account_code") && call.contains("&as_of"),
        "RED: cross-module owned string → demoted &str must borrow:\n{rs}"
    );
    test.cargo_check().expect("cross-module cargo-check");
}
