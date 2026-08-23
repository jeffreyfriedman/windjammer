#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! FAILING REPRO — module-level `const FOO: string = "lit"` used as `-> string` return
//! must codegen as owned `String`, not `&str`.
//!
//! Ecosystem `wj-mime` hit E0308: `expected String, found &str` when returning `JSON`
//! from `pub fn json() -> string { JSON }` and in match arms mixing literals with consts.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn module_const_string_return_must_emit_owned_string() {
    let generated = test_utils::compile_single(
        r#"
const JSON: string = "application/json; charset=utf-8"

pub fn json() -> string {
    JSON
}
"#,
    );

    assert!(
        !generated.contains("return JSON;"),
        "const string return must not pass &str through unchanged:\n{generated}"
    );
    let owns = generated.contains("JSON.to_string()")
        || generated.contains("String::from(JSON)")
        || generated.contains("JSON.clone()");
    assert!(
        owns,
        "const string return must convert to owned String:\n{generated}"
    );
}

#[test]
fn module_const_string_match_arm_must_unify_with_string_literals() {
    let generated = test_utils::compile_single(
        r#"
const JSON: string = "application/json; charset=utf-8"

pub fn from_ext(ext: string) -> string {
    match ext {
        "json" => JSON,
        _ => "application/octet-stream",
    }
}
"#,
    );

    assert!(
        !generated.contains("=> JSON,") || generated.contains("JSON.to_string()"),
        "match arm using module const must unify with String literal arms:\n{generated}"
    );
}
