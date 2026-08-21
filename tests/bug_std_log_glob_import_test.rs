//! `use std::log::*` must import runtime log functions, not only alias the module.
//!
//! Ecosystem `wj-log`: glob import of `std::log` was emitting
//! `use windjammer_runtime::log_mod as log;` (functions stay out of scope → E0425).
//! HTTP already globs (`use windjammer_runtime::http::*;`) because `http` is not a `*_mod` stem.

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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn std_log_glob_imports_runtime_functions() {
    let source = r#"
use std::log::*

pub fn announce(msg: string) {
    info(msg)
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("use windjammer_runtime::log_mod::*;"),
        "std::log glob must import log_mod items, got:\n{generated}"
    );
    assert!(
        !generated.contains("use windjammer_runtime::log_mod as log;"),
        "glob must not only alias the module (functions stay out of scope):\n{generated}"
    );
    assert!(
        generated.contains("info("),
        "info() call must remain in generated body:\n{generated}"
    );
}

#[test]
fn std_log_error_borrows_string_like_info() {
    // Homonyms (`http::error`, `dialog::error`) must not steal `log_mod::error`'s `&str`.
    let source = r#"
use std::log::*

pub fn emit(tag: string, message: string) {
    let line = "[${tag}] ${message}"
    error(line)
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("error(&line)"),
        "imported std::log error takes &str; owned local must be borrowed, got:\n{generated}"
    );
}
