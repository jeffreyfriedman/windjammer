#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

#[path = "common/test_utils.rs"]
mod test_utils;

/// strings::split(line, "|") must pass a string literal delimiter as &str, not String.
#[test]
fn test_strings_split_pipe_delimiter_codegen() {
    let source = r##"
use std::strings

pub fn split_pipe(line: string) -> Vec<string> {
    strings::split(line, "|")
}
"##;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains(r#"strings::split("#),
        "expected strings::split call in:\n{}",
        generated
    );
    assert!(
        !generated.contains(r#""|".to_string()"#),
        "pipe delimiter must not be coerced to String. Generated:\n{}",
        generated
    );
    assert!(
        !generated.contains(r#""|","#) || generated.contains(r#", "|""#),
        "delimiter should remain a string literal. Generated:\n{}",
        generated
    );
}
