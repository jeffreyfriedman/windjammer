//! `strings.contains(haystack, owned_needle)` must demote `String` → `&str`.
//!
//! Ecosystem first-hour hit E0308 when codegen passed `format!("\"{}\"", key)`
//! by value into `strings::contains(..., substring: &str)`.

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
fn strings_contains_owned_interpolated_needle_must_cargo_check() {
    let source = r#"
use std::strings

pub fn has_quoted_key(card: string, key: string) -> bool {
    strings.contains(card, "\"${key}\"")
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["strings::contains"]);
}
