//! TDD test for method call auto-clone bug.
//!
//! Bug: When calling a method on a borrowed reference inside a for loop,
//! the codegen inserts .clone() between the method name and parentheses:
//!   e.get_tag.clone()()  ← WRONG (treats method as field, clones it, calls result)
//! Should be:
//!   e.get_tag()          ← CORRECT (just call the method)
//!
//! This affects: match expr on method calls, method calls in for loops over &Vec

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_wj(source: &str) -> String {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp_dir = std::env::temp_dir().join(format!("wj_method_clone_test_{}_{}", std::process::id(), id));
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).unwrap();
    
    let source_path = tmp_dir.join("test.wj");
    std::fs::write(&source_path, source).unwrap();
    
    let output_dir = tmp_dir.join("output");
    let _output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", source_path.to_str().unwrap(), "--target", "rust", "--output", output_dir.to_str().unwrap(), "--no-cargo"])
        .output()
        .expect("failed to run wj");
    
    let rs_path = output_dir.join("test.rs");
    let generated = std::fs::read_to_string(&rs_path)
        .unwrap_or_else(|_| panic!("Failed to read generated Rust at {:?}", rs_path));
    
    let _ = std::fs::remove_dir_all(&tmp_dir);
    generated
}

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
    let generated = compile_wj(source);
    
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
    let generated = compile_wj(source);
    
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
