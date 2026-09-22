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

//! Owned struct formal moved into `Vec::push` must stay owned — not demote to `&mut T`.
//!
//! Ecosystem `wj-todo-cli` `insert_by_id`: compares `item.id`, splits `sorted`, then
//! `push(item)`. Simplified `out.push(item)`-only fixtures false-green on tip.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn owned_item_pushed_must_not_demote_to_mut_ref() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", r#"pub mod query"#);
    test.add_file(
        "query.wj",
        r#"
pub struct Todo {
    pub id: int,
    pub title: string,
    pub done: bool,
}

fn insert_by_id(sorted: Vec<Todo>, item: Todo) -> Vec<Todo> {
    let mut before = Vec::new()
    let mut after = Vec::new()
    let mut placed = false
    for todo in sorted {
        if !placed && item.id < todo.id {
            placed = true
        }
        if placed {
            after.push(todo)
        } else {
            before.push(todo)
        }
    }
    let mut out = before
    out.push(item)
    for todo in after {
        out.push(todo)
    }
    out
}

pub fn sort_todos_by_id(todos: Vec<Todo>) -> Vec<Todo> {
    let mut sorted = Vec::new()
    for todo in todos {
        sorted = insert_by_id(sorted, todo)
    }
    sorted
}
"#,
    );

    test.cargo_check().unwrap_or_else(|e| {
        panic!("owned item moved into push must stay owned (not &mut), cargo-check:\n{e}")
    });

    let map = test.compile().expect("recompile for assert");
    let query_rs = map.get("query.rs").expect("query.rs");
    assert!(
        !query_rs.contains("item: &mut Todo") && !query_rs.contains("item: & mut Todo"),
        "insert_by_id item formal must not demote to &mut Todo, got:\n{query_rs}"
    );
    assert!(
        !query_rs.contains("insert_by_id(&sorted, &mut todo)"),
        "call site must pass owned todo into owned item formal, got:\n{query_rs}"
    );
}
