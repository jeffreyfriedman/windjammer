//! Cross-crate calls into `wj-querystring` must auto-borrow owned `string` → `&str`.
//!
//! Ecosystem `wj-url` thin-wrap attempt: `qs_get(query, key)` emits E0308
//! (`expected &str, found String`) when querystring formals are demoted Borrowed.

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
fn cross_crate_owned_string_into_borrowed_formal_must_cargo_check() {
    // Minimal same-crate stand-in: owned args into &str formals (mirrors demoted package API).
    let source = r#"
use std::strings

fn takes_borrowed(hay: string, needle: string) -> bool {
    strings.contains(hay, needle)
}

pub fn wrap(query: string, key: string) -> bool {
    takes_borrowed(query, key)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["strings::contains"]);
}
