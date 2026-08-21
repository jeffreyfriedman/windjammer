//! Reused owned `string` locals must clone into a second owned-string callee (E0308).
//!
//! Ecosystem `wj-sitegen`: `generate_page` calls `render_document(title, body)` and
//! also stores `title` on the struct. Codegen emitted `markdown_to_html(&markdown)` /
//! `render_document(title)` after a prior move.

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
fn reused_owned_string_second_callee_must_not_borrow() {
    let source = r#"
fn consume(s: string) -> string {
    s
}

pub fn twice(s: string) -> string {
    let a = consume(s)
    let b = consume(s)
    a + b
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("consume(s.clone())")
            || generated.contains("let b = consume(s.clone())"),
        "second owned-string use must clone, got:\n{generated}"
    );
    assert!(
        !generated.contains("consume(&s)"),
        "must not pass &String into owned string formal, got:\n{generated}"
    );
}
