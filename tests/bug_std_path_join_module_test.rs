//! `std::path` join / file_name must reach runtime and `cargo check`.
//!
//! Runtime `path::join` returns `PathBuf`; std surface is `string` — wiring must
//! convert (or std must document PathBuf). Ecosystem `wj-path` is a workaround.

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
fn std_path_join_codegen_resolves() {
    let source = r#"
use std::path

pub fn combine(a: string, b: string) -> string {
    path.join(a, b)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["path::join"]);
}

#[test]
fn std_path_file_name_codegen_resolves() {
    let source = r#"
use std::path

pub fn name(p: string) -> string {
    match path.file_name(p) {
        Some(n) => n,
        None => "",
    }
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["path::file_name"]);
}
