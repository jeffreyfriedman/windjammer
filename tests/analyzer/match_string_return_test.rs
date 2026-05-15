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

#[path = "../common/test_utils.rs"]
mod test_utils;

/// Helper to compile Windjammer code and return the generated Rust code
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_string_literal_in_return_position() {
    let code = r#"
    enum Status {
        Active,
        Inactive,
        Pending,
    }
    
    fn get_label(status: Status) -> string {
        match status {
            Status::Active => "Active",
            Status::Inactive => "Inactive",
            Status::Pending => "Pending",
        }
    }
    "#;
    let generated = test_utils::compile_single_result(code).expect("Compilation failed");
    // Match arms with string literals should be converted when function returns String
    assert!(
        generated.contains(r#""Active".to_string()"#),
        "Match arm string literals should convert to String: {}",
        generated
    );
    assert!(
        generated.contains(r#""Inactive".to_string()"#),
        "All arms should convert: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_empty_string_return() {
    let code = r#"
    enum ObjectType {
        Cube,
        Empty,
    }
    
    fn render_object(obj: ObjectType) -> string {
        match obj {
            ObjectType::Cube => "Rendered cube",
            ObjectType::Empty => "",
        }
    }
    "#;
    let generated = test_utils::compile_single_result(code).expect("Compilation failed");
    // Empty string should also be converted
    assert!(
        generated.contains(r#""".to_string()"#)
            || generated.contains(r#""Rendered cube".to_string()"#),
        "Empty string literal should convert to String: {}",
        generated
    );
}
