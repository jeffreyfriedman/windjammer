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

//! TDD Test: If-else branches should have consistent String types
//!
//! When one branch returns a string literal and another returns a field,
//! both should be String for consistency.
//!
//! NOTE: All tests in this file spawn subprocesses (cargo run) which are very slow
//! under tarpaulin instrumentation, so they're skipped in coverage runs.

#[path = "common/test_utils.rs"]
mod test_utils;

// Skip in coverage - spawns subprocess (very slow under tarpaulin)
#[cfg_attr(tarpaulin, ignore)]
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_else_literal_vs_field() {
    // If one branch returns literal, the other returns field
    let code = r#"
pub struct Item {
    name: string,
}

impl Item {
    pub fn display_name(self) -> string {
        if self.name == "" {
            "Unnamed"
        } else {
            self.name
        }
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
        "If-else with literal vs field should compile. Error: {}",
        err
    );
}

// Skip in coverage - spawns subprocess (very slow under tarpaulin)
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_else_both_literals() {
    // Both branches return literals
    let code = r#"
pub fn get_status(active: bool) -> string {
    if active {
        "Active"
    } else {
        "Inactive"
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
        "If-else with both literals should compile. Error: {}",
        err
    );
}

// Skip in coverage - spawns subprocess (very slow under tarpaulin)
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_else_field_vs_literal() {
    // Reversed: field first, literal second
    let code = r#"
pub struct Config {
    value: string,
}

impl Config {
    pub fn get_value(self) -> string {
        if self.value != "" {
            self.value
        } else {
            "default"
        }
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
        "If-else with field vs literal should compile. Error: {}",
        err
    );
}

// Skip in coverage - spawns subprocess (very slow under tarpaulin)
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_if_else_strings() {
    // Nested if-else with strings
    let code = r#"
pub fn classify(code: i32) -> string {
    if code == 0 {
        "Success"
    } else if code < 100 {
        "Minor error"
    } else if code < 500 {
        "Major error"
    } else {
        "Critical"
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
        "Nested if-else with strings should compile. Error: {}",
        err
    );
}
