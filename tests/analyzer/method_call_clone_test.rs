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

//! TDD test for method call auto-clone bug.
//!
//! Bug: When calling a method on a borrowed reference inside a for loop,
//! the codegen inserts .clone() between the method name and parentheses:
//!   e.get_tag.clone()()  ← WRONG (treats method as field, clones it, calls result)
//! Should be:
//!   e.get_tag()          ← CORRECT (just call the method)
//!
//! This affects: match expr on method calls, method calls in for loops over &Vec

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_method_call_on_borrowed_ref_in_for_loop() {
    let source = r#"
struct Item {
    pub name: String,
    pub value: i32,
}

impl Item {
    fn new(name: String, value: i32) -> Item {
        Item { name: name, value: value }
    }
    
    fn get_name(self) -> &String {
        &self.name
    }
}

fn main() {
    let items = vec![
        Item::new("A".to_string(), 1),
        Item::new("B".to_string(), 2),
    ]
    
    for item in &items {
        println!("{}", item.get_name())
    }
}
"#;
    let generated = test_utils::compile_single(source);

    // Should NOT have .clone() between method name and parentheses
    assert!(
        !generated.contains(".get_name.clone()"),
        "Should not insert .clone() between method name and call parens.\nGenerated:\n{}",
        generated
    );

    // Should have proper method call syntax
    assert!(
        generated.contains(".get_name()"),
        "Should generate proper method call.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_method_call_in_match_on_borrowed_ref() {
    let source = r#"
struct Entity {
    pub name: String,
    pub health: f32,
}

impl Entity {
    fn new(name: String, health: f32) -> Entity {
        Entity { name: name, health: health }
    }
    
    fn get_health(self) -> Option<f32> {
        if self.health > 0.0 {
            Some(self.health)
        } else {
            None
        }
    }
}

fn main() {
    let entities = vec![
        Entity::new("Player".to_string(), 100.0),
        Entity::new("Dead".to_string(), 0.0),
    ]
    
    for e in &entities {
        match e.get_health() {
            Some(hp) => println!("{} has {} HP", e.name, hp),
            None => println!("{} is dead", e.name),
        }
    }
}
"#;
    let generated = test_utils::compile_single(source);

    // Should NOT have .clone() between method name and parentheses
    assert!(
        !generated.contains(".get_health.clone()"),
        "Should not insert .clone() between method name and call parens.\nGenerated:\n{}",
        generated
    );

    // Should have proper method call syntax
    assert!(
        generated.contains(".get_health()"),
        "Should generate proper method call.\nGenerated:\n{}",
        generated
    );
}
