//! Ecosystem `wj-json-util` needs named `Value` parameters and `json::keys`.
//! Runtime must publicly re-export `serde_json::Value` and expose object keys;
//! `use std::json` must import that type (same pattern as `regex` → `Regex`).

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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn json_value_type_imported_with_std_json() {
    let source = r#"
use std::json

pub fn tag(value: Value) -> bool {
    json.is_object(value)
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("use windjammer_runtime::json::Value;"),
        "use std::json must import public Value type, got:\n{generated}"
    );
}

#[test]
fn json_keys_emits_runtime_call_with_borrow() {
    let source = r#"
use std::json

pub fn object_key_count(value: Value) -> int {
    let keys = json.keys(value)
    keys.len()
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("json::keys(&value)") || generated.contains("json::keys(value)"),
        "expected json::keys call, got:\n{generated}"
    );
}

#[test]
fn http_response_type_imported_with_std_http() {
    let source = r#"
use std::http

pub fn status(r: Response) -> int {
    r.status_code() as int
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("use windjammer_runtime::http::Response;"),
        "use std::http must import public Response type, got:\n{generated}"
    );
}
