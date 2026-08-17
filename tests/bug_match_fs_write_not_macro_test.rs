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

//! FAILING REPRO — `match fs.write(...)` must not lower to `write!(...)`.
//!
//! Match scrutinees parse `fs.write(...)` as `Call(FieldAccess)`, and early
//! macro dispatch treated bare name `write` as the Rust `write!` macro.
//! Ecosystem `wj-dotenv` uses `match fs.write(...)` in package tests.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn match_fs_write_codegens_fs_write_not_write_macro() {
    let generated = test_utils::compile_single(
        r#"
use std::fs

pub fn write_fixture(path: string) -> Result<(), string> {
    match fs.write(path, "APP=ecosystem\n") {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
"#,
    );

    assert!(
        !generated.contains("write!("),
        "fs.write in match must not become write! macro:\n{generated}"
    );
    assert!(
        generated.contains("fs::write(") || generated.contains("std::fs::write("),
        "expected fs::write call site:\n{generated}"
    );
}
