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

//! Cross-module match-arm calls into multi-use `string` formals.
//!
//! Two valid outcomes under multipass (signature / IR driven):
//! - **Read-only concat** (`json + ""`): callee demotes to `&str`; call site auto-borrows.
//! - **True ownership** (identity move / explicit clone): callee keeps `String`; call site moves.
//!
//! Both must reject the WDB-110 class bug: `&arg.clone()` into an owned `String` formal.
//!
//! Dogfood (`finance-screens`): public ports pass bare `json`; helpers that only read demote.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn assert_no_borrow_clone_into_owned_string_formal(consumer: &str, callee: &str, call: &str) {
    assert!(
        !consumer.contains(&format!("{call}(&")),
        "owned String formal must not receive borrow at cross-module match-arm call site.\ncallee=\n{callee}\nconsumer=\n{consumer}"
    );
    assert!(
        !consumer.contains(&format!("{call}(&json.clone()"))
            && !consumer.contains(&format!("{call}(& json.clone()")),
        "must not emit &json.clone() into owned String formal (WDB-110).\ncallee=\n{callee}\nconsumer=\n{consumer}"
    );
}

#[test]
fn cross_module_match_arm_readonly_concat_demotes_to_str() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "parser.wj",
        r#"
use std::strings

pub fn parse_twice(json: string) -> int {
    let _ = strings.len(json + "")
    let _ = strings.len(json + "")
    0
}
"#,
    );
    test.add_file(
        "consumer.wj",
        r#"
use crate::parser::parse_twice

pub fn dispatch(kind: string, json: string) -> int {
    match kind {
        "schema" => parse_twice(json),
        _ => 0,
    }
}
"#,
    );
    test.add_file(
        "main.wj",
        r#"
use crate::consumer::dispatch

fn main() {
    let _ = dispatch("schema", "{}" + "")
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let consumer = map.get("consumer.rs").expect("consumer.rs output");
    let parser = map.get("parser.rs").expect("parser.rs output");

    assert!(
        parser.contains("json: &str"),
        "read-only concat helpers demote to &str. Got:\n{parser}"
    );
    assert!(
        consumer.contains("parse_twice(json)"),
        "demoted &str formal must receive bare auto-borrow, not explicit &. Got:\n{consumer}"
    );
    assert_no_borrow_clone_into_owned_string_formal(consumer, parser, "parse_twice");

    test.cargo_check()
        .expect("read-only concat cross-module call must cargo check");
}

#[test]
fn cross_module_match_arm_multi_use_owned_formal_must_move_not_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "parser.wj",
        r#"
pub fn own_json(s: string) -> string {
    s
}

pub fn parse_twice(json: string) -> int {
    let _ = own_json(json.clone())
    let _ = own_json(json)
    0
}
"#,
    );
    test.add_file(
        "consumer.wj",
        r#"
use crate::parser::parse_twice

pub fn dispatch(kind: string, json: string) -> int {
    match kind {
        "schema" => parse_twice(json),
        _ => 0,
    }
}
"#,
    );
    test.add_file(
        "main.wj",
        r#"
use crate::consumer::dispatch

fn main() {
    let _ = dispatch("schema", "{}" + "")
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let consumer = map.get("consumer.rs").expect("consumer.rs output");
    let parser = map.get("parser.rs").expect("parser.rs output");

    assert!(
        parser.contains("json: String"),
        "multi-use owned consumption keeps String formal. Got:\n{parser}"
    );
    assert!(
        consumer.contains("parse_twice(json)"),
        "owned String formal must move at cross-module match-arm call site. Got:\n{consumer}"
    );
    assert_no_borrow_clone_into_owned_string_formal(consumer, parser, "parse_twice");

    test.cargo_check()
        .expect("multi-use owned formal cross-module call must cargo check");
}
