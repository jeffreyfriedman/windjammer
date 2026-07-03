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

//! TDD: typed struct serialization via json.to_string()
//!
//! Windjammer auto-derives Serialize/Deserialize for all structs when
//! the project uses std::json. No @derive annotation needed.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn json_to_string_serializes_struct() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "main.wj",
        r#"
use std::json

struct UserProfile {
    name: string,
    age: int,
    active: bool,
}

pub fn serialize_user() -> Result<string, string> {
    let user = UserProfile {
        name: "Alice",
        age: 30,
        active: true,
    }
    json.to_string(user)
}

pub fn roundtrip_test() -> bool {
    let user = UserProfile {
        name: "Bob",
        age: 25,
        active: false,
    }
    let text = match json.to_string(user) {
        Ok(s) => s,
        Err(_) => return false,
    }
    text.len() > 0
}
"#,
    );
    test.assert_compiles_without_error();
}

#[test]
fn json_to_string_pretty_serializes_with_indentation() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "main.wj",
        r#"
use std::json

struct Config {
    host: string,
    port: int,
}

pub fn pretty_config() -> Result<string, string> {
    let cfg = Config {
        host: "localhost",
        port: 8080,
    }
    json.to_string_pretty(cfg)
}
"#,
    );
    test.assert_compiles_without_error();
}

#[test]
fn json_serialize_deserialize_roundtrip() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "main.wj",
        r#"
use std::json

struct Point {
    x: int,
    y: int,
}

pub fn roundtrip() -> Result<Point, string> {
    let p = Point { x: 10, y: 20 }
    let text = json.to_string(p)?
    json.parse_string(text)
}
"#,
    );
    test.assert_compiles_without_error();
}
