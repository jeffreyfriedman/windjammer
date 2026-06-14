#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! TDD Test: Match arms with format! should convert string literals
//!
//! When one match arm uses format!() and another uses a string literal,
//! the string literal should be converted to String for type consistency.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_with_format_and_literal() {
    let code = r#"
fn render_label(value: Option<f32>) -> string {
    let label = match value {
        Some(v) => format!("{:.2}", v),
        None => "N/A",
    }
    label
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Compile error:\n{}", err);
    }

    assert!(success, "Compilation should succeed");

    assert!(
        generated.contains("\"N/A\".to_string()")
            || generated.contains("String::from(\"N/A\")"),
        "String literal in match arm should be converted to String. Generated:\n{}",
        generated
    );
}
