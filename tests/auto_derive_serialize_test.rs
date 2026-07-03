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

//! TDD: All Windjammer structs should auto-derive Serialize and Deserialize.
//!
//! The Windjammer philosophy: "compiler does the hard work, not the developer."
//! Just as Copy/Clone/Debug are auto-inferred, Serialize/Deserialize should be too.
//! Users should never need @derive(Serialize) — that's Rust leakage.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn plain_struct_auto_derives_serialize() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "main.wj",
        r#"
use std::json

pub struct Account {
    pub code: string,
    pub name: string,
    pub balance_cents: int,
}

pub fn serialize_account() -> Result<string, string> {
    let acct = Account {
        code: "1000",
        name: "Cash",
        balance_cents: 50000,
    }
    json.to_string(acct)
}
"#,
    );
    test.assert_compiles_without_error();
}

#[test]
fn nested_struct_auto_derives_serialize() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "main.wj",
        r#"
use std::json

pub struct LineItem {
    pub code: string,
    pub amount: int,
}

pub struct Report {
    pub title: string,
    pub lines: Vec<LineItem>,
}

pub fn serialize_report() -> Result<string, string> {
    let report = Report {
        title: "Q1",
        lines: vec![
            LineItem { code: "100", amount: 500 },
        ],
    }
    json.to_string(report)
}
"#,
    );
    test.assert_compiles_without_error();
}

#[test]
fn struct_in_project_with_json_import_auto_derives_serialize() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "main.wj",
        r#"
use std::json

pub struct Config {
    pub host: string,
    pub port: int,
}

pub fn to_json(cfg: Config) -> Result<string, string> {
    json.to_string(cfg)
}
"#,
    );
    test.assert_contains("main.rs", "Serialize");
    test.assert_compiles_without_error();
}
