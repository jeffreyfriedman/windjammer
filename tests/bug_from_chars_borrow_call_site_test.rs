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

//! `strings.from_chars(chars)` — runtime takes `&[char]` but codegen may

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn from_chars_owned_vec_must_borrow_at_call_site() {
    let source = r#"
use std::strings

pub fn rebuild(chars: Vec<char>) -> string {
    strings.from_chars(chars)
}
"#;

    let rs = test_utils::compile_single(source);
    assert!(
        rs.contains("from_chars(&chars") || rs.contains("from_chars(& chars"),
        "owned Vec<char> into borrowed from_chars must auto-borrow. Got:\n{rs}"
    );
    assert!(
        !rs.contains("from_chars(chars)") || rs.contains("from_chars(&chars"),
        "must not move Vec<char> into & formal. Got:\n{rs}"
    );
}
