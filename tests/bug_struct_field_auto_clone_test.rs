// TDD Test for Bug: Struct field access in loops incorrectly auto-cloned
// 
// Bug: When accessing a String field from a struct reference in a loop,
// the compiler incorrectly adds .clone() even when the receiving function
// expects a &String reference.
//
// Source:    for ingredient in &self.ingredients {
//                if !inventory.has_item(ingredient.item_id, quantity) {
// Generated: if !inventory.has_item(ingredient.item_id.clone(), quantity) {
//                                                         ^^^^^^^^ INCORRECT!
//
// Expected:  if !inventory.has_item(&ingredient.item_id, quantity) {
//                                    ^ Should add & not .clone()

use std::process::Command;
use std::fs;

fn compile_wj_test(source: &str) -> (bool, String, String) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_test_{}", timestamp));
    fs::create_dir_all(&temp_dir).unwrap();
    
    let source_file = temp_dir.join("test.wj");
    fs::write(&source_file, source).unwrap();
    
    let output_dir = temp_dir.join("out");
    
    let output = Command::new("wj")
        .args(&["build", source_file.to_str().unwrap()])
        .args(&["--output", output_dir.to_str().unwrap()])
        .args(&["--target", "rust"])
        .args(&["--no-cargo"])
        .output()
        .expect("Failed to run wj");
    
    let _success = output.status.success();
    
    // Read generated Rust code
    let rust_file = output_dir.join("test.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap_or_else(|_| String::from("(file not generated)"));
    
    // Try to compile with rustc to check for errors
    let rustc_output = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&rust_file)
        .arg("--out-dir")
        .arg(&temp_dir)
        .output()
        .expect("Failed to run rustc");
    
    let stderr = String::from_utf8_lossy(&rustc_output.stderr).to_string();
    
    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
    
    (rustc_output.status.success(), rust_code, stderr)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_field_in_loop_no_auto_clone() {
    let source = r#"
pub struct Item {
    pub id: string,
    pub quantity: i32,
}

pub struct Inventory {
    items: Vec<Item>,
}

impl Inventory {
    pub fn has_item(&self, item_id: string, quantity: i32) -> bool {
        for item in &self.items {
            if item.id == item_id && item.quantity >= quantity {
                return true
            }
        }
        false
    }
}

fn main() {
    let inv = Inventory { items: Vec::new() }
    let result = inv.has_item("sword", 1)
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    
    if !success {
        panic!("Compilation failed:\n{}\n\nGenerated code:\n{}", stderr, rust_code);
    }
    
    // Should NOT have .clone() on struct field access
    // Source: item.id == item_id
    // Should generate: item.id == item_id (or &item.id == &item_id)
    // Should NOT generate: item.id.clone() == item_id
    assert!(!rust_code.contains("item.id.clone()"), 
            "Should not auto-clone struct field in comparison:\n{}", rust_code);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_field_passed_to_borrowed_param() {
    let source = r#"
pub struct Ingredient {
    pub item_id: string,
    pub quantity: i32,
}

pub struct Recipe {
    ingredients: Vec<Ingredient>,
}

impl Recipe {
    pub fn check_inventory(&self, has_item: fn(string, i32) -> bool) -> bool {
        for ingredient in &self.ingredients {
            if !has_item(ingredient.item_id, ingredient.quantity) {
                return false
            }
        }
        true
    }
}

fn dummy_has_item(item_id: string, quantity: i32) -> bool {
    true
}

fn main() {
    let recipe = Recipe { ingredients: Vec::new() }
    let result = recipe.check_inventory(dummy_has_item)
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    
    if !success {
        panic!("Compilation failed:\n{}\n\nGenerated code:\n{}", stderr, rust_code);
    }
    
    // Compiler should infer correct borrowing/ownership
    // Since has_item takes `string` (inferred as &String in params),
    // the compiler should pass &ingredient.item_id, NOT ingredient.item_id.clone()
    assert!(!rust_code.contains("ingredient.item_id.clone()"), 
            "Should not auto-clone struct field when passing to borrowed param:\n{}", rust_code);
}
