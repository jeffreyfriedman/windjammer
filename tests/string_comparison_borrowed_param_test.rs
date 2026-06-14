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

/// TDD Test: String comparisons with borrowed &String parameters
///
/// PROBLEM: self.field == borrowed_param fails when field is String and param is &String
/// SOLUTION: Auto-add * deref for &String in comparisons, or use .as_str() for both sides
///
/// This test validates that String comparisons work correctly after changing
/// borrowed parameters from &str to &String
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_string_field_equals_borrowed_param() {
    // Test: self.field == param where field is String and param is &String
    let code = r#"
pub struct Player {
    pub name: string,
}

impl Player {
    pub fn has_name(self, n: string) -> bool {
        self.name == n
    }
}

pub fn main() {
    let p = Player { name: "Alice" }
    let result = p.has_name("Alice")
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "String field == borrowed &String param should work:\n{:?}",
        result.err()
    );
}

#[test]
fn test_borrowed_param_equals_string_field() {
    // Test: param == self.field (reversed operands)
    let code = r#"
pub struct Player {
    pub name: string,
}

impl Player {
    pub fn has_name(self, n: string) -> bool {
        n == self.name
    }
}

pub fn main() {
    let p = Player { name: "Alice" }
    let result = p.has_name("Alice")
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Borrowed &String param == String field should work:\n{:?}",
        result.err()
    );
}

#[test]
fn test_string_comparison_in_complex_expression() {
    // Test: String comparisons in &&, || expressions
    let code = r#"
pub struct Player {
    pub name: string,
    pub title: string,
}

impl Player {
    pub fn matches(self, n: string, t: string) -> bool {
        self.name == n && self.title == t
    }
}

pub fn main() {
    let p = Player { name: "Alice", title: "Knight" }
    let result = p.matches("Alice", "Knight")
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Multiple String comparisons should work:\n{:?}",
        result.err()
    );
}

#[test]
fn test_string_comparison_with_match_arm_binding() {
    // Test: Match arm binding (String) compared with field (String)
    let code = r#"
pub struct Inventory {
    pub items: Vec<string>,
}

pub enum Condition {
    HasItem(string),
}

impl Condition {
    pub fn check(self, inv: Inventory) -> bool {
        match self {
            Condition::HasItem(item_id) => {
                // This compares string (from match arm) with string (from Vec)
                inv.items.any(|stored| stored == item_id)
            }
        }
    }
}

pub fn main() {
    let inv = Inventory { items: Vec::new() }
    let cond = Condition::HasItem("sword")
    let result = cond.check(inv)
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Match arm binding comparison should work:\n{:?}",
        result.err()
    );
}
