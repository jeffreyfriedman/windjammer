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

//! Multipass demotes read-only `encode_line(todo: Todo)` to `&Todo`; same-file
//! `encode_store` must auto-borrow the loop item (`&item` / `&item.clone()`),
//! not pass owned `item.clone()` (E0308).
//!
//! Ecosystem `wj-todo-cli` `src/domain/codec.wj` `encode_store`.
//! Related: `bug_wdb125_*`, `bug_todo_cli_demoted_struct_loop_must_not_star_deref_test`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn demoted_encode_line_clone_must_auto_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod domain
"#,
    );
    test.add_file(
        "domain/mod.wj",
        r#"
pub mod todo
pub mod codec
"#,
    );
    test.add_file(
        "domain/todo.wj",
        r#"
pub struct Todo {
    pub id: int,
    pub title: string,
    pub done: bool,
}

pub struct TodoStore {
    items: Vec<Todo>,
}

impl TodoStore {
    pub fn from_todos(todos: Vec<Todo>) -> TodoStore {
        TodoStore { items: todos }
    }

    pub fn into_todos(self) -> Vec<Todo> {
        self.items
    }
}
"#,
    );
    test.add_file(
        "domain/codec.wj",
        r#"
use crate::domain::Todo
use crate::domain::TodoStore

pub fn encode_line(todo: Todo) -> string {
    let flag = if todo.done {
        "1"
    } else {
        "0"
    }
    "${todo.id}\t${flag}\t${todo.title}"
}

pub fn encode_store(store: TodoStore) -> string {
    let todos = store.into_todos()
    let mut out = ""
    let mut first = true
    for item in todos {
        if !first {
            out = "${out}\n"
        }
        first = false
        out = "${out}${encode_line(item)}"
    }
    out
}

pub fn sample() -> string {
    encode_store(TodoStore::from_todos(vec![Todo {
        id: 1,
        title: "x",
        done: false,
    }]))
}
"#,
    );

    test.cargo_check().unwrap_or_else(|e| {
        panic!(
            "encode_line demoted &Todo + encode_store loop must auto-borrow (not item.clone()):\n{e}"
        )
    });

    let map = test.compile().expect("recompile for assert");
    let codec_rs = map
        .get("domain/codec.rs")
        .or_else(|| map.get("codec.rs"))
        .expect("codec.rs");
    let demoted = codec_rs.contains("todo: &Todo") || codec_rs.contains("todo: & Todo");
    let borrows = codec_rs.contains("encode_line(&item")
        || codec_rs.contains("encode_line(& item")
        || codec_rs.contains("encode_line(&item.clone()")
        || codec_rs.contains("encode_line(& item.clone()");
    let bad_owned = codec_rs.contains("encode_line(item.clone())") && !borrows;
    assert!(
        !demoted || !bad_owned,
        "must not pass owned item.clone() into demoted &Todo:\n{codec_rs}"
    );
}
