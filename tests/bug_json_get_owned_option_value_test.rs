//! `std::json` declares `get` / `get_index` as `Option<Value>` (owned).
//! Runtime must return owned values so WJ helpers can `return json.get(...)`.

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
fn json_get_return_as_owned_option_value_compiles() {
    let source = r#"
use std::json

pub fn take_field(value: Value, key: string) -> Option<Value> {
    json.get(value, key)
}

pub fn take_index(value: Value, index: usize) -> Option<Value> {
    json.get_index(value, index)
}
"#;
    let generated = test_utils::compile_single(source);
    // Must not leave bare Option<&Value> in a Option<Value> return slot.
    assert!(
        !generated.contains("-> Option<&Value>")
            && !generated.contains("-> Option<&json::Value>"),
        "WJ Option<Value> must not emit Option<&Value> return, got:\n{generated}"
    );
    assert!(
        generated.contains("json::get(") && generated.contains("json::get_index("),
        "expected get/get_index calls, got:\n{generated}"
    );
}
