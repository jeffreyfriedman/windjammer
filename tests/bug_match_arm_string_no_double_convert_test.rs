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

/// Match arms returning string literals must use a single conversion — never double-convert.
#[test]
fn test_match_arm_string_literal_no_double_convert() {
    let source = r##"
pub enum Level {
    Low,
    High,
}

impl Level {
    pub fn label(self) -> string {
        match self {
            Level::Low => "low",
            Level::High => "high",
        }
    }
}
"##;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("\"low\".to_string()") || generated.contains("\"low\".into()"),
        "expected string conversion for match arm literal:\n{}",
        generated
    );
    assert!(
        !generated.contains(".into().to_string()"),
        "must not double-convert match arm strings:\n{}",
        generated
    );
    assert!(
        !generated.contains(".to_string().to_string()"),
        "must not double-convert match arm strings:\n{}",
        generated
    );
}
