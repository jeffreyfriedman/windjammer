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

/// std::strings parse/split/chars APIs must codegen to windjammer_runtime::strings (no Rust leakage).
#[test]
fn test_std_strings_parse_api_compiles() {
    let source = r##"
use std::strings

pub fn exercise_strings_api(text: string) -> string {
    let lines = strings.split_lines(text)
    let parts = strings.split_whitespace("a b")
    let _ = strings.parse_i32("-1")
    let _ = strings.parse_f32("1.5")
    let _ = strings.parse_bool("true")
    let _ = strings.byte_at("X", 0)
    let joined = strings.join(parts, "-")
    let codepoints = strings.chars("Hi")
    let rebuilt = strings.from_chars(codepoints)
    let slice = strings.substring_chars("Hello", 1, 4)
    let trimmed = strings.trim("  x ")
    let _ = strings.starts_with("ab", "a")
    let _ = strings.is_empty("")
    if lines.len() > 0 {
        return rebuilt + slice + trimmed + joined
    }
    "empty"
}
"##;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("strings::split_lines"),
        "expected strings::split_lines in:\n{}",
        generated
    );
    assert!(
        generated.contains("strings::chars"),
        "expected strings::chars in:\n{}",
        generated
    );
    assert!(
        generated.contains("strings::from_chars"),
        "expected strings::from_chars in:\n{}",
        generated
    );
    assert!(
        !generated.contains(".to_string()"),
        "string API usage must not emit .to_string() in:\n{}",
        generated
    );
    assert!(
        !generated.contains(".as_bytes()"),
        "must not emit .as_bytes() in:\n{}",
        generated
    );
}
