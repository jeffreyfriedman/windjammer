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

//! WDB-106: explicit `.clone()` on owned string passed to first of two sequential
//! calls must emit `.clone()` in generated Rust (Phase 156 tbl pipe-field parse).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn wdb106_explicit_clone_on_first_of_two_owned_string_calls_must_emit() {
    let source = r#"
pub fn pipe_field(line: string, index: usize) -> string {
    let chars = strings::chars(line)
    let mut field = 0 as usize
    let mut start = 0 as usize
    let mut i = 0 as usize
    while i < chars.len() {
        if chars[i] == '|' {
            if field == index {
                return strings::substring(line, start, i)
            }
            field = field + 1
            start = i + 1
        }
        i = i + 1
    }
    if field == index {
        return strings::substring(line, start, chars.len())
    }
    ""
}

pub fn parse_two_fields(line: string) -> (string, string) {
    let ok = pipe_field(line.clone(), 0)
    let qty = pipe_field(line, 4)
    (ok, qty)
}
"#;

    let rs = test_utils::compile_single(source);
    assert!(
        rs.contains("line.clone()"),
        "WDB-106: explicit clone before second use of owned string must emit. Got:\n{rs}"
    );
}

#[test]
fn wdb106_explicit_clone_for_is_empty_before_move_must_emit() {
    let source = r#"
pub fn use_after_empty_check(line: string) -> bool {
    if strings::is_empty(line.clone()) {
        return false
    }
    let trimmed = strings::trim(line)
    strings::len(trimmed) > 0
}
"#;

    let rs = test_utils::compile_single(source);
    assert!(
        rs.contains("line.clone()"),
        "WDB-106: explicit clone for is_empty before move must emit. Got:\n{rs}"
    );
}
