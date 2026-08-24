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

//! FAILING REPRO — `map.insert(...)` as the sole expression in an `if`/`else` arm must be `()`.
//!
//! Ecosystem `wj-todo-cli` hit E0308: expected `()`, found `Option<Todo>` when codegen kept
//! `insert`'s Option as the arm value. Workaround: `let _ = map.insert(...)`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn hashmap_insert_in_if_arm_must_be_unit() {
    let generated = test_utils::compile_single(
        r#"
use std::collections::HashMap

pub fn put(map: HashMap<int, string>, key: int, value: string, replace: bool) -> HashMap<int, string> {
    let mut out = map
    if replace {
        out.insert(key, value)
    }
    out
}
"#,
    );

    assert!(
        generated.contains("let _ =") && generated.contains("insert"),
        "insert in if-arm should discard Option (let _ = insert(...)):\n{generated}"
    );
}
