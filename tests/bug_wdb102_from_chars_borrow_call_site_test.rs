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

//! WDB-102: `strings.from_chars(chars)` — runtime takes `&[char]` but codegen may
//! pass owned `Vec<char>` (Phase 148 `relational_pg_wire_port` dogfood: `from_chars(&chars)`).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn wdb102_from_chars_owned_vec_must_borrow_at_call_site() {
    let source = r#"
use std::strings

pub fn rebuild(chars: Vec<char>) -> string {
    strings.from_chars(chars)
}
"#;

    let rs = test_utils::compile_single(source);
    assert!(
        rs.contains("from_chars(&chars") || rs.contains("from_chars(& chars"),
        "WDB-102: owned Vec<char> into borrowed from_chars must auto-borrow. Got:\n{rs}"
    );
    assert!(
        !rs.contains("from_chars(chars)") || rs.contains("from_chars(&chars"),
        "WDB-102: must not move Vec<char> into & formal. Got:\n{rs}"
    );
}
