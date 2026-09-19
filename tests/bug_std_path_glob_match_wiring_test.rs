//! `std::path.glob_match` must wire for shell-style `*` / `?` / `**` matching.
//!
//! Ecosystem `wj-glob` is pure-WJ today. Prefer std once this resolves so fs walk
//! and static site tools share one matcher.

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
fn path_glob_match_star_codegen_resolves() {
    let source = r#"
use std::path

pub fn md_only(name: string) -> bool {
    path.glob_match("*.md", name)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["path::glob_match"]);
}

#[test]
fn path_glob_match_recursive_codegen_resolves() {
    let source = r#"
use std::path

pub fn under_src(p: string) -> bool {
    path.glob_match("src/**/*.wj", p)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["path::glob_match"]);
}
