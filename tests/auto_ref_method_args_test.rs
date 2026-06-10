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

//! TDD Test: Auto-reference method arguments when method expects &T but gets T
//!
//! When a method expects &Vec<T> or &Option<T> but receives Vec<T> or Option<T>,
//! the compiler should automatically add &.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_ref_vec_arg() {
    // Parameter inferred as borrowed Vec when not mutated
    let code = r#"
pub fn process_items(items: Vec<i32>) -> i32 {
    let mut sum: i32 = 0
    for item in items {
        sum = sum + item
    }
    sum
}

pub fn test() -> i32 {
    let items: Vec<i32> = vec![1, 2, 3]
    process_items(items)
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
        "Generated code should compile. Error:\n{}\nGenerated:\n{}",
        err, generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_ref_option_arg() {
    // Method expects &Option<String> but we pass Option<String>
    let code = r#"
pub fn display_optional(value: Option<string>) -> string {
    match value {
        Some(s) => s.clone(),
        None => "empty".to_string(),
    }
}

pub fn test() -> string {
    let maybe_name = Some("Alice".to_string())
    display_optional(maybe_name)
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Generated code should compile. Error: {}", err);
}
