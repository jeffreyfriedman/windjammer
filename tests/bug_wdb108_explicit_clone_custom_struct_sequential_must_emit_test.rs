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

//! WDB-108: explicit `.clone()` on sequential owned custom-struct / Vec reuse
//! (Phase 167 PEG `parse_ast_with_registry`) must emit in generated Rust.
//! WDB-106 covered owned `string`; PRE still drops clones for `WdbSqlParser`-shaped locals.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn wdb108_explicit_clone_on_sequential_owned_custom_struct_calls_must_emit() {
    let source = r#"
pub struct Cursor {
    pub pos: int,
}

pub fn advance(c: Cursor) -> Cursor {
    Cursor { pos: c.pos + 1 }
}

pub fn try_a(c: Cursor) -> (Cursor, bool) {
    if c.pos == 0 {
        return (c, true)
    }
    (c, false)
}

pub fn try_b(c: Cursor) -> (Cursor, bool) {
    if c.pos == 1 {
        return (c, true)
    }
    (c, false)
}

pub fn parse_with_fallback(c: Cursor) -> int {
    let a = try_a(c.clone())
    if a.1 {
        return a.0.pos
    }
    let b = try_b(c.clone())
    if b.1 {
        return b.0.pos
    }
    let last = advance(c)
    last.pos
}
"#;

    let rs = test_utils::compile_single(source);
    assert!(
        rs.contains("c.clone()"),
        "WDB-108: explicit clone before sequential owned custom-struct calls must emit. Got:\n{rs}"
    );
}

#[test]
fn wdb108_explicit_clone_on_vec_before_parser_and_cursor_reuse_must_emit() {
    let source = r#"
pub struct Tok {
    pub n: int,
}

pub struct Parser {
    pub tokens: Vec<Tok>,
    pub pos: int,
}

pub fn parser_new(tokens: Vec<Tok>) -> Parser {
    Parser { tokens: tokens, pos: 0 }
}

pub fn cursor_from(tokens: Vec<Tok>, pos: int) -> int {
    tokens.len() + pos
}

pub fn parse_ast(tokens: Vec<Tok>) -> int {
    let parser = parser_new(tokens.clone())
    let _ = parser.pos
    cursor_from(tokens.clone(), 0)
}
"#;

    let rs = test_utils::compile_single(source);
    assert!(
        rs.contains("tokens.clone()"),
        "WDB-108: explicit clone of Vec before parser + cursor reuse must emit. Got:\n{rs}"
    );
}
