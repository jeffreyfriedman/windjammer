//! `std::regex` string APIs must reach runtime and `cargo check`.
//!
//! Ecosystem `wj-regex` wraps is_match / find / replace_all / split / escape.

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
fn std_regex_is_match_codegen_resolves() {
    let source = r#"
use std::regex

pub fn check(pattern: string, text: string) -> Result<bool, string> {
    regex.is_match(pattern, text)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["regex::is_match"]);
}

#[test]
fn std_regex_find_all_codegen_resolves() {
    let source = r#"
use std::regex

pub fn all(pattern: string, text: string) -> Result<Vec<string>, string> {
    regex.find_all(pattern, text)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["regex::find_all"]);
}

#[test]
fn std_regex_escape_codegen_resolves() {
    let source = r#"
use std::regex

pub fn lit(text: string) -> string {
    regex.escape(text)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["regex::escape"]);
}
