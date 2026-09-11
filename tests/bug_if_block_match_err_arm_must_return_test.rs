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

//! Trailing `match` with `Err(e) => Err(e)` inside `if` must compile as function return.
//! `wj-todo-cli` `parse_command` `edit` branch class.

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const PARSE_EDIT: &str = include_str!("fixtures/library_multipass/parse_edit_if_match_err.wj");

#[test]
fn if_block_match_err_arm_must_compile_as_function_return() {
    let generated = test_utils::compile_single(PARSE_EDIT);
    assert!(
        generated.contains("return Ok(") || generated.contains("return Err("),
        "if-block trailing match must emit return; got:\n{generated}"
    );
}

#[test]
fn if_block_match_err_arm_multipass_must_cargo_check() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod commands
pub use commands::parse_edit_branch
"#,
    );
    project.add_file("commands.wj", PARSE_EDIT);

    project
        .compile()
        .expect("parse_edit_branch multipass compile should succeed")
        .get("commands.rs")
        .expect("commands.rs");

    project.cargo_check().expect_err(
        "RED: if-block match Err arm multipass must fail cargo-check until return unify is fixed",
    );
}
