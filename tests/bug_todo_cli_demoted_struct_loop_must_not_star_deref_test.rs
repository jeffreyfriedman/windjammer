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

//! Multipass demotes read-only owned struct formal to `&T`; for-loop call sites must not
//! emit `*item` into that formal (E0308: expected `&Todo`, found `Todo`).
//!
//! Ecosystem `wj-todo-cli`: `encode_line(todo: Todo)` → `&Todo`, `encode_line(*todo)` in render loop.
//! Related: `bug_wdb125_module_file_demoted_struct_formal_must_borrow_clone_call_sites_test`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn demoted_struct_loop_call_must_not_star_deref() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod codec
pub mod run
"#,
    );
    test.add_file(
        "codec.wj",
        r#"
pub struct Todo {
    pub id: int,
    pub title: string,
}

pub fn encode_line(todo: Todo) -> string {
    "${todo.id}\t${todo.title}"
}
"#,
    );
    test.add_file(
        "run.wj",
        r#"
use crate::codec::Todo
use crate::codec::encode_line

pub fn render(todos: Vec<Todo>) -> string {
    let mut out = ""
    for todo in todos {
        let line = encode_line(todo)
        out = "${out}${line}\n"
    }
    out
}
"#,
    );

    test.cargo_check().unwrap_or_else(|e| {
        panic!("demoted struct loop call must cargo-check (no *todo into &Todo):\n{e}")
    });

    let map = test.compile().expect("recompile for assert");
    let run_rs = map.get("run.rs").expect("run.rs");
    assert!(
        !run_rs.contains("encode_line(*todo)") && !run_rs.contains("encode_line(* todo)"),
        "must not star-deref loop binding into demoted &Todo formal, got:\n{run_rs}"
    );
}
