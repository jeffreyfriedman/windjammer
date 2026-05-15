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

//! TDD Test: String concatenation with function/method calls
//!
//! FIXED: String + String concatenation now correctly borrows the right side.
//!
//! Pattern: `result = result + process_property()` where process_property returns String
//! Solution: Automatic `&` prefix added to right side: `result = result + &process_property()`
//!
//! Implementation:
//! 1. Enhanced infer_expression_type() to detect String returns from:
//!    - Function calls (func() -> String)
//!    - Method calls (self.method() -> String)
//!    - Macro invocations (format!() -> String)
//!
//! 2. Disabled compound assignment (+=) when right side is String
//!    - Checks right_type only (not target_type) for robustness
//!    - Works even when target type can't be inferred
//!
//! 3. Added automatic borrowing in binary expressions
//!    - String + String → String + &String
//!    - Skips string literals (already &str)

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_borrowed_item_field_access() {
    // When iterating over borrowed items, fields need .clone()
    let code = r#"
pub struct Property {
    pub name: string,
    pub value: string,
}

pub fn process_property(name: string, value: string) -> string {
    format!("{}: {}", name, value)
}

pub fn process_properties(props: &Vec<Property>) -> string {
    let mut result = "".to_string()
    for prop in props {
        result = result + process_property(prop.name, prop.value)
    }
    result
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    // The prop.name and prop.value should have .clone() added
    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_call_with_borrowed_fields() {
    // Method calls with borrowed item fields
    let code = r#"
pub struct Item {
    pub label: string,
    pub description: string,
}

pub struct Display {
    items: Vec<Item>,
}

impl Display {
    pub fn render_item(&self, label: string, description: string) -> string {
        format!("<div>{}: {}</div>", label, description)
    }
    
    pub fn render_all(&self) -> string {
        let mut result = "".to_string()
        for item in self.items {
            result = result + self.render_item(item.label, item.description)
        }
        result
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
        "Method call with borrowed fields should compile. Error: {}",
        err
    );
}
