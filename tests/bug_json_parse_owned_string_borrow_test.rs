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

//! FAILING REPRO — `json.parse(body)` with owned `string` must emit `&body` (runtime takes `&str`).
//! Ecosystem `wj-fetch` `format_body` hits this in a match that also returns owned `body`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn json_parse_owned_string_auto_borrows() {
    let generated = test_utils::compile_single(
        r#"
use std::json

pub fn format_body(body: string) -> string {
    match json.parse(body) {
        Ok(value) => {
            match json.stringify_pretty(value) {
                Ok(pretty) => pretty,
                Err(_) => body,
            }
        },
        Err(_) => body,
    }
}
"#,
    );

    assert!(
        !generated.contains("json::parse(body.clone())")
            && !generated.contains("json::parse(body)"),
        "json::parse must borrow owned string:\n{generated}"
    );
    assert!(
        generated.contains("json::parse(&body")
            || generated.contains("json::parse(body.as_str())"),
        "expected borrowed parse arg:\n{generated}"
    );
}
