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

//! Cross-module match-arm call to multi-use owned `string` formal must move, not borrow.
//!
//! Dogfood (`finance-screens` `read_models.wj` → `parse_analytics_schema_fields`):
//! when the callee reuses an owned `json` param, tip must not emit `&json.clone()` at the
//! call site while the Rust formal stays `String` (E0308).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn cross_module_match_arm_multi_use_owned_formal_must_move_not_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "parser.wj",
        r#"
pub fn parse_twice(json: string) -> int {
    let a = strings.len(json + "")
    let b = strings.len(json + "")
    a + b
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
        "repro needs owned String formal. Got:\n{parser}"
    );
    assert!(
        !consumer.contains("parse_twice(&"),
        "owned String formal must not receive borrow at cross-module match-arm call site.\nparser=\n{parser}\nconsumer=\n{consumer}"
    );

    test.cargo_check()
        .expect("multi-use owned formal cross-module call must cargo check");
}
