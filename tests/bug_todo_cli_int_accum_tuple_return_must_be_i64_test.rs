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

//! Untyped `let mut n = 0` accumulators with `-> (int, int, int)` must codegen as i64.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn int_accum_tuple_return_must_cargo_check_as_i64() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", r#"pub mod query
pub mod store"#);
    test.add_file(
        "query.wj",
        r#"
pub struct Todo { pub id: int, pub done: bool }

pub fn count_todos(todos: Vec<Todo>) -> (int, int, int) {
    let mut pending = 0
    let mut done = 0
    for todo in todos {
        if todo.done { done = done + 1 } else { pending = pending + 1 }
    }
    (pending + done, pending, done)
}
"#,
    );
    test.add_file(
        "store.wj",
        r#"
pub struct Todo { pub id: int, pub done: bool }

pub fn merge_from(start: int, extra: Vec<Todo>) -> (int, int) {
    let mut merged = 0
    for _item in extra { merged = merged + 1 }
    (start + merged, merged)
}
"#,
    );
    test.cargo_check().unwrap_or_else(|e| panic!("i64 accumulators:\n{e}"));
    let map = test.compile().expect("recompile");
    let query_rs = map.get("query.rs").expect("query.rs");
    assert!(
        !query_rs.contains("pending: i32") && !query_rs.contains("done: i32"),
        "count_todos accumulators must be i64, got:\n{query_rs}"
    );
}
