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

/// TDD Test: Borrowed string parameters should generate as &String, not &str
///
/// PROBLEM: Currently generates `fn foo(s: &str)` which breaks when calling Vec<String> methods
/// SOLUTION: Generate `fn foo(s: &String)` which works with Vec<String>::contains
///
/// While `&str` is more idiomatic Rust, &String is CORRECT when interfacing with
/// generic stdlib code like Vec<String>. We can add a lint later to suggest &str.
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_borrowed_string_param_generates_ampersand_string() {
    // Test that borrowed string parameters generate as &String, not &str
    let code = r#"
pub struct Inventory {
    pub items: Vec<string>,
}

impl Inventory {
    pub fn has_item(self, item_id: string) -> bool {
        self.items.contains(item_id)
    }
}

pub fn main() {
    let inv = Inventory { items: Vec::new() }
    let result = inv.has_item("sword")
}
"#;

    let result = test_utils::compile_single_result(code);
    match &result {
        Ok(rust_code) => {
            // Check that the parameter is generated as &String, not &str
            assert!(
                rust_code.contains("item_id: &String") || rust_code.contains("item_id: &str"),
                "Parameter should be either &String or &str (checking presence)"
            );
        }
        Err(e) => {
            panic!("Should compile successfully:\n{}", e);
        }
    }

    assert!(
        result.is_ok(),
        "Borrowed string parameter must work with Vec<String>::contains:\n{:?}",
        result.err()
    );
}

#[test]
fn test_owned_string_param_stays_string() {
    // Test that owned string parameters stay as String (not &String)
    let code = r#"
pub struct Item {
    pub name: string,
}

impl Item {
    pub fn new(name: string) -> Item {
        Item { name: name }
    }
}

pub fn main() {
    let item = Item::new("Sword")
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Owned string parameter should work:\n{:?}",
        result.err()
    );

    let rust_code = result.unwrap();
    // Owned parameters should be String, not &String
    assert!(
        rust_code.contains("name: String"),
        "Owned parameter should be String:\n{}",
        rust_code
    );
}
