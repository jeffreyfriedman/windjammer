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

//! FAILING REPRO — `HashMap.get("lit")` after `parse` Result match (ecosystem `wj-cookie`).
//!
//! Closely related to `bug_hashmap_get_literal_in_match_arm_test`, but the call site is
//! a package-shaped `match parse_cookie_header(...) { Ok(map) => map.get("a") }` where
//! codegen emits `get("a".to_string())` and rustc E0308's (`expected &_`, found String).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn hashmap_get_literal_after_result_parse_stays_borrowed() {
    let generated = test_utils::compile_single(
        r#"
use std::collections::HashMap

pub fn parse_pairs(text: string) -> Result<HashMap<string, string>, string> {
    let mut map = HashMap::new()
    map.insert("a", "1")
    Ok(map)
}

pub fn read_a(text: string) -> string {
    match parse_pairs(text) {
        Ok(map) => {
            match map.get("a") {
                Some(v) => v,
                None => "",
            }
        },
        Err(_) => "",
    }
}
"#,
    );

    assert!(
        !generated.contains("get(\"a\".to_string())")
            && !generated.contains("get(\"a\".to_owned())"),
        "HashMap::get key literal must not be owned after Result match:\n{generated}"
    );
    assert!(
        generated.contains("get(\"a\")"),
        "expected borrowed string literal key:\n{generated}"
    );
}
