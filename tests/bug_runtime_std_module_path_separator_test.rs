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
))]

//! Runtime std modules (`process`, `http`, …) must emit Rust path `::`, not `.`.
//! Driven by `is_runtime_std_module` (scanned std module names), not method-name lists.
//! Ecosystem `wj-fetch` uses `process.exit(1)` and `http.get(url)`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn process_exit_emits_path_separator_not_method_dot() {
    let generated = test_utils::compile_single(
        r#"
use std::process

pub fn die() {
    process.exit(1)
}
"#,
    );
    assert!(
        generated.contains("process::exit(1)"),
        "process.exit must lower to process::exit:\n{generated}"
    );
    assert!(
        !generated.contains("process.exit("),
        "must not emit instance-method dot for runtime std module:\n{generated}"
    );
}

#[test]
fn io_is_terminal_emits_path_separator_for_unlisted_scanned_module() {
    let generated = test_utils::compile_single(
        r#"
use std::io

pub fn tty() -> bool {
    io.is_terminal()
}
"#,
    );
    assert!(
        generated.contains("io::is_terminal()"),
        "scanned io module (not a hardcoded name) must emit path :: :\n{generated}"
    );
    assert!(
        !generated.contains("io.is_terminal("),
        "must not emit instance-method dot for scanned runtime std module:\n{generated}"
    );
}

#[test]
fn http_get_emits_path_separator() {
    let generated = test_utils::compile_single(
        r#"
use std::http

pub fn ping(url: string) -> Result<u16, string> {
    match http.get(url) {
        Ok(response) => Ok(response.status_code()),
        Err(e) => Err(e),
    }
}
"#,
    );
    assert!(
        generated.contains("http::get("),
        "http.get must lower to http::get:\n{generated}"
    );
}
