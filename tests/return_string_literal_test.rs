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

//! TDD Test: Return statements with string literals need .to_string()
//!
//! When returning a string literal from a function that returns String,
//! the literal should be converted automatically.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_empty_string() {
    // Return empty string literal
    let code = r#"
pub fn get_default() -> string {
    return ""
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(
        generated.contains(r#""".to_string()"#) || generated.contains("String::new()"),
        "Empty return should convert to String. Generated:\n{}",
        generated
    );
    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_string_literal() {
    // Return non-empty string literal
    let code = r#"
pub fn get_message() -> string {
    return "Hello"
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(
        generated.contains(r#""Hello".to_string()"#)
            || generated.contains(r#"string::from("Hello")"#)
            || generated.contains(r#"String::from("Hello")"#),
        "Return should convert to String. Generated:\n{}",
        generated
    );
    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_early_return_string_literal() {
    // Early return with string literal
    let code = r#"
pub struct Widget {
    visible: bool,
}

impl Widget {
    pub fn render(self) -> string {
        if !self.visible {
            return ""
        }
        return "<div>Visible</div>"
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(
        success,
        "Early return with string literal should compile. Error: {}",
        err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conditional_return_strings() {
    // Multiple conditional returns
    let code = r#"
pub fn status_message(code: i32) -> string {
    if code == 0 {
        return "Success"
    }
    if code == 1 {
        return "Error"
    }
    return "Unknown"
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(
        success,
        "Conditional returns should compile. Error: {}",
        err
    );
}
