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

//! TDD: String literals inside if-else expressions used as struct field values
//! must get `.to_string()` when the field type is String.
//!
//! Bug: `Property { value: if cond { "true" } else { "false" } }` generates
//! bare `"true"` / `"false"` instead of `"true".to_string()` / `"false".to_string()`.
//!
//! Root cause: Struct literal codegen only auto-converts direct Expression::Literal
//! to `.to_string()`, but doesn't propagate coercion into if-else branch bodies.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_if_else_string_literal_in_struct_field() {
    let source = r#"
pub struct Property {
    pub name: string,
    pub value: string
}

pub fn boolean_prop(name: string, value: bool) -> Property {
    Property {
        name,
        value: if value { "true" } else { "false" },
    }
}
"#;
    let rust = test_utils::compile_single(source);

    assert!(
        rust.contains(r#""true".to_string()"#) || rust.contains(r#"string::from("true")"#) || rust.contains(r#"String::from("true")"#),
        "Expected owned \"true\" in if-else struct field. Got:\n{}",
        rust
    );
    assert!(
        rust.contains(r#""false".to_string()"#) || rust.contains(r#"string::from("false")"#) || rust.contains(r#"String::from("false")"#),
        "Expected owned \"false\" in if-else struct field. Got:\n{}",
        rust
    );
}

#[test]
fn test_if_else_empty_string_in_struct_field() {
    let source = r#"
pub struct Item {
    pub label: string
}

pub fn make_item(cond: bool) -> Item {
    Item {
        label: if cond { "hello" } else { "" },
    }
}
"#;
    let rust = test_utils::compile_single(source);

    assert!(
        rust.contains(r#""hello".to_string()"#) || rust.contains(r#"string::from("hello")"#) || rust.contains(r#"String::from("hello")"#),
        "Expected owned \"hello\". Got:\n{}",
        rust
    );
    assert!(
        rust.contains(r#""".to_string()"#) || rust.contains("String::new()"),
        "Expected owned empty string. Got:\n{}",
        rust
    );
}
