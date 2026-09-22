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

//! Untyped `let mut n = 0` accumulators returned as `int` / `(int, int, int)` must be i64.
//!
//! Ecosystem `wj-todo-cli` `count_todos` / `merge_from`: E0308 `expected i64, found i32`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn int_accum_tuple_return_must_cargo_check_as_i64() {
    let source = r#"
pub struct Item {
    pub done: bool,
}

pub fn count_items(items: Vec<Item>) -> (int, int, int) {
    let mut pending = 0
    let mut done = 0
    for item in items {
        if item.done {
            done = done + 1
        } else {
            pending = pending + 1
        }
    }
    (pending + done, pending, done)
}

pub fn merge_count(start: int, extra: Vec<Item>) -> (int, int) {
    let mut merged = 0
    for _item in extra {
        merged = merged + 1
    }
    (start + merged, merged)
}
"#;
    let (generated, ok) = test_utils::compile_single_check(source);
    assert!(
        ok,
        "int accumulator into int/tuple return must cargo-check as i64, got:\n{generated}"
    );
}
