//! Empty string literals inside `Ok((…, ""))` must own as `String`.
//!
//! Ecosystem `wj-url`: `split_authority` returns `Ok((text, ""))`.
//! Bare `(text, "")` already emits `String::new()`, but wrapping in `Ok(...)`
//! leaves a bare `&str` (E0308).

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
fn empty_string_literal_in_ok_tuple_owns_string() {
    let source = r#"
pub fn split_tail(text: string) -> Result<(string, string), string> {
    Ok((text, ""))
}
"#;
    let generated = test_utils::compile_single(source);
    let owns = generated.contains("String::new()")
        || generated.contains("\"\".to_string()")
        || generated.contains("String::from(\"\")");
    assert!(
        owns,
        "Ok((text, \"\")) must own empty string, got:\n{generated}"
    );
    assert!(
        !generated.contains("Ok((text, \"\"))"),
        "must not leave bare &str \"\" in Ok tuple, got:\n{generated}"
    );
}
