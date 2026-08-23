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

//! FAILING REPRO — module-level `const string` returned from `-> string` fn codegen as `&str`.
//!
//! Ecosystem `wj-mime` hit E0308: `const JSON: string = "..."` then `return JSON` emitted `&str`
//! instead of `String`. Workaround: inline string literals in returns / match arms.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn module_const_string_return_must_emit_owned_string() {
    let generated = test_utils::compile_single(
        r#"
const LABEL: string = "application/json"

pub fn json_type() -> string {
    LABEL
}
"#,
    );

    assert!(
        !generated.contains("return LABEL;"),
        "const string return must coerce to owned String, got bare const ref:\n{generated}"
    );
}
