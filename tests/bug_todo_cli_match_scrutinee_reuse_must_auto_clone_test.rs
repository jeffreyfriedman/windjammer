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

//! Match scrutinee move + arm reuse must auto-clone (`render_json_and_restore`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn match_scrutinee_move_and_arm_reuse_must_auto_clone() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"pub mod codec
pub mod run
"#,
    );
    test.add_file(
        "codec.wj",
        r#"
pub struct Todo { pub id: int, pub title: string, pub done: bool }
pub fn encode_line(todo: Todo) -> string { "${todo.id}" }
pub fn decode_line(line: string) -> Result<Todo, string> {
    Ok(Todo { id: 1, title: "x", done: false })
}
pub fn todos_to_json(todos: Vec<Todo>) -> Result<string, string> { Ok("[]") }
"#,
    );
    test.add_file(
        "run.wj",
        r#"
use crate::codec::Todo
use crate::codec::encode_line
use crate::codec::decode_line
use crate::codec::todos_to_json

pub struct TodoStore { items: Vec<Todo> }
impl TodoStore {
    pub fn from_todos(todos: Vec<Todo>) -> TodoStore { TodoStore { items: todos } }
}

pub fn render_json_and_restore(todos: Vec<Todo>) -> Result<(TodoStore, string), string> {
    let mut restored = Vec::new()
    for todo in todos {
        let line = encode_line(todo)
        match decode_line(line) {
            Ok(item) => restored.push(item),
            Err(e) => return Err(e),
        }
    }
    match todos_to_json(restored) {
        Ok(out) => Ok((TodoStore::from_todos(restored), out)),
        Err(e) => Err(e),
    }
}
"#,
    );
    test.cargo_check().unwrap_or_else(|e| panic!("E0382 match reuse:\n{e}"));
    let map = test.compile().expect("recompile");
    let run_rs = map.get("run.rs").expect("run.rs");
    assert!(
        run_rs.contains("restored.clone()"),
        "must clone restored at scrutinee, got:\n{run_rs}"
    );
}
