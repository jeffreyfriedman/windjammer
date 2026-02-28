/// TDD Test: Accessing owned field from borrowed struct
/// 
/// Problem: ingredient.item_id where ingredient: &Ingredient, item_id: String
///          generates: ingredient.item_id.clone()
///          but method expects: &String
/// Error: expected `&String`, found `String`
/// 
/// Root Cause: When accessing owned field from borrowed struct, compiler adds .clone()
///             but method signature expects &String
/// 
/// Solution: When passing borrowed_struct.owned_field to method expecting &T,
///           pass &borrowed_struct.owned_field (no clone)

use std::fs;
use std::process::Command;

#[test]
fn test_borrowed_struct_owned_field_to_ref() {
    let source = r#"
struct Ingredient {
    item_id: string,
    quantity: i32
}

struct Inventory {}

impl Inventory {
    pub fn has_item(self, item_id: string, quantity: i32) -> bool {
        item_id.len() > 0
    }
}

fn check_item(inv: Inventory, item: Ingredient) -> bool {
    inv.has_item(item.item_id, item.quantity)
}

fn main() {
    let inv = Inventory {}
    let item = Ingredient { item_id: "sword", quantity: 1 }
    let result = check_item(inv, item)
    println("{}", result)
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!("wj_test_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos());
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();
    
    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();
    
    let out_dir = test_dir.join("out");
    
    let _output = Command::new("wj")
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .output()
        .expect("Failed to run wj compiler");
    
    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file)
        .expect("Failed to read generated Rust file");
    
    println!("Generated code:\n{}", generated);
    
    let rustc_output = Command::new("rustc")
        .arg(&rust_file)
        .arg("--crate-type")
        .arg("bin")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test_bin"))
        .output()
        .expect("Failed to run rustc");
    
    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!("Compilation failed:\n{}\n\nGenerated code:\n{}", stderr, generated);
    }
    
    // Verify: borrowed_struct.owned_field passed as &field when method expects &String
    // Should be: inv.has_item(&item.item_id, ...) 
    // NOT: inv.has_item(item.item_id.clone(), ...)
    assert!(
        generated.contains("has_item(&item.item_id,") ||
        generated.contains("has_item(&item.item_id )"),
        "Should pass &borrowed_struct.owned_field when method expects &String"
    );
    
    fs::remove_dir_all(&test_dir).ok();
}
