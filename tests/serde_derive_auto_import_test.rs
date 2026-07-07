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

//! TDD: When a project uses `use std::json`, all structs auto-derive Serialize/Deserialize.
//! No explicit @derive(Serialize) needed — the compiler detects json usage and applies it.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn struct_with_json_import_auto_derives_serialize() {
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

pub fn serialize(acct: Account) -> Result<string, string> {
    json.to_string(acct)
}
"#,
    );
    test.assert_compiles_without_error();
}

#[test]
fn struct_with_json_import_auto_derives_deserialize() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "main.wj",
        r#"
use std::json

pub struct LoginRequest {
    pub email: string,
    pub password: string,
}

pub fn parse(body: string) -> Result<LoginRequest, string> {
    json.parse_string(body)
}
"#,
    );
    test.assert_compiles_without_error();
}

#[test]
fn json_import_enables_serde_import_in_generated_code() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "main.wj",
        r#"
use std::json

pub struct Point {
    pub x: int,
    pub y: int,
}
"#,
    );
    test.assert_contains("main.rs", "use serde::");
    test.assert_contains("main.rs", "Serialize");
}
