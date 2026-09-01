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

//! FAILING REPRO — multipass `domain/codec.wj` calling `json.is_array(root)` / `json.len(root)`
//! after `json.parse` must `cargo check`. Runtime takes `&Value`; multipass emits owned `Value` → E0308.
//! Ecosystem `wj-todo-cli` `todos_from_json` hit this; workaround uses `[` prefix + `get_index` loop.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn json_is_array_len_owned_value_multipass_must_cargo_check() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod codec
pub use codec::array_len
"#,
    );
    project.add_file(
        "codec.wj",
        r#"
use std::json

pub fn array_len(text: string) -> int {
    match json.parse(text) {
        Ok(root) => {
            if !json.is_array(root) {
                return 0
            }
            json.len(root)
        },
        Err(_) => 0,
    }
}
"#,
    );

    project
        .cargo_check()
        .expect_err("multipass json.is_array/json.len on owned Value must fail cargo check until borrow codegen is fixed");
}
