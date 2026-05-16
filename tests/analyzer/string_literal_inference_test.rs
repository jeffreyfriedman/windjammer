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

// Test: String literal inference in various contexts
// The compiler should automatically convert "literal" to String when context expects String

#[path = "../common/test_utils.rs"]
mod test_utils;

/// Helper to compile Windjammer code and return the generated Rust code
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_arm_string_inference() {
    let code = r#"
        fn get_status(opt: Option<i32>) -> string {
            match opt {
                Some(x) => "has value",
                None => "empty",
            }
        }
    "#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Match arms should infer .to_string() for string literals"
    );

    let generated = result.unwrap();

    // Should generate .to_string() automatically
    assert!(
        generated.contains("\"has value\".to_string()"),
        "Expected match arm to convert string literal to String"
    );
    assert!(
        generated.contains("\"empty\".to_string()"),
        "Expected match arm to convert string literal to String"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_match_string_inference() {
    let code = r#"
        fn get_class(selected: Option<string>, id: string) -> string {
            match selected {
                Some(sel_id) => if sel_id == id { "selected" } else { "normal" },
                None => "normal",
            }
        }
    "#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Nested if-else in match should infer .to_string()"
    );

    let generated = result.unwrap();
    assert!(generated.contains("\"selected\".to_string()"));
    assert!(generated.contains("\"normal\".to_string()"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_else_string_inference() {
    let code = r#"
        fn get_status(is_active: bool) -> string {
            if is_active { "active" } else { "inactive" }
        }
    "#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "If-else should infer .to_string() when returning string"
    );

    let generated = result.unwrap();
    assert!(generated.contains("\"active\".to_string()"));
    assert!(generated.contains("\"inactive\".to_string()"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_field_string_inference() {
    let code = r#"
        struct Config {
            name: string,
            parent: Option<string>,
        }
        
        fn new_config() -> Config {
            Config {
                name: "default",
                parent: Some("root"),
            }
        }
    "#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Struct fields should infer .to_string() for string literals"
    );

    let generated = result.unwrap();
    assert!(generated.contains("name: \"default\".to_string()"));
    assert!(generated.contains("Some(\"root\".to_string()"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_option_some_string_inference() {
    let code = r#"
        fn get_parent() -> Option<string> {
            Some("parent_id")
        }
        
        fn get_none() -> Option<string> {
            None
        }
    "#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Option::Some should infer .to_string()");

    let generated = result.unwrap();
    assert!(generated.contains("Some(\"parent_id\".to_string())"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_result_string_inference() {
    let code = r#"
        fn validate(value: i32) -> Result<string, string> {
            if value > 0 {
                Ok("valid")
            } else {
                Err("invalid")
            }
        }
    "#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Result Ok/Err should infer .to_string()");

    let generated = result.unwrap();
    assert!(generated.contains("Ok(\"valid\".to_string())"));
    assert!(generated.contains("Err(\"invalid\".to_string())"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_ternary_like_match_string_inference() {
    let code = r#"
        fn get_label(is_root: bool) -> string {
            if is_root { "🌟 Root" } else { "" }
        }
    "#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Ternary-like if-else should infer .to_string()"
    );

    let generated = result.unwrap();
    assert!(generated.contains("\"🌟 Root\".to_string()"));
    assert!(generated.contains("\"\".to_string()"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_with_blocks_string_inference() {
    let code = r#"
        fn process(value: Option<i32>) -> string {
            match value {
                Some(x) => {
                    if x > 10 {
                        "large"
                    } else {
                        "small"
                    }
                },
                None => "empty",
            }
        }
    "#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Match arms with blocks should infer .to_string()"
    );

    let generated = result.unwrap();
    assert!(generated.contains("\"large\".to_string()"));
    assert!(generated.contains("\"small\".to_string()"));
    assert!(generated.contains("\"empty\".to_string()"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_no_inference_for_str_return() {
    let code = r#"
        fn get_static() -> &str {
            "static"
        }
    "#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Should NOT infer .to_string() when returning &str"
    );

    let generated = result.unwrap();
    assert!(
        !generated.contains("\"static\".to_string()"),
        "Should not convert to String when &str is expected"
    );
    assert!(generated.contains("\"static\""));
}
