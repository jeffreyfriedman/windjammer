#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

#[path = "../common/test_utils.rs"]
mod test_utils;

/// Untyped `let` with string literal or string match must emit `: String` for `.into()` to resolve.
#[test]
fn test_untyped_let_string_ascription_compiles() {
    let source = r##"
pub fn labels() -> string {
    let en_code = "en"
    let tag = match 1 {
        0 => "[ok]",
        _ => "[!!]",
    }
    en_code + tag
}
"##;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("let en_code: String = \"en\".into()"),
        "string literal let needs : String ascription:\n{}",
        generated
    );
    assert!(
        generated.contains("let tag: String = match"),
        "string match let needs : String ascription:\n{}",
        generated
    );
    assert!(
        !generated.contains(".into().to_string()"),
        "must not double-convert:\n{}",
        generated
    );
}
