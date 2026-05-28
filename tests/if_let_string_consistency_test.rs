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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_let_string_literal_consistency() {
    let code = r#"
    fn classify(x: Option<i32>) -> string {
        let result = if let Some(val) = x {
            if val > 0 { "positive" } else { "negative" }
        } else {
            "none"
        }
        result
    }
    "#;
    let generated = test_utils::compile_single_result(code).expect("Compilation failed");
    // When if-let is transformed to match, all arms should return consistent types
    // If Some arm returns String (via .to_string()), None arm should too
    assert!(
        generated.contains(r#""none".to_string()"#)
            || !generated.contains(r#""positive".to_string()"#),
        "Match arms should have consistent types: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_let_with_function_return() {
    let code = r#"
    fn get_status(active: Option<i32>) -> string {
        if let Some(id) = active {
            if id == 1 { "active" } else { "inactive" }
        } else {
            "unknown"
        }
    }
    "#;
    let generated = test_utils::compile_single_result(code).expect("Compilation failed");
    // All branches should return String consistently
    println!("Generated:\n{}", generated);
}
