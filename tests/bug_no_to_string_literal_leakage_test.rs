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

/// String literals are `string` — user code must not need `.to_string()`.
#[test]
fn test_string_literal_without_to_string_compiles() {
    let source = r##"
pub fn greet(name: string) -> string {
    name
}

pub fn make_label() -> string {
    "Hello World"
}

pub fn empty_label() -> string {
    ""
}
"##;

    let generated = test_utils::compile_single(source);
    assert!(
        !generated.contains(".to_string()"),
        "string literals should not emit .to_string() in generated Rust:\n{}",
        generated
    );
}
