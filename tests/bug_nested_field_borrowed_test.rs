/// TDD Test: Nested Field Access Through Borrowed Iterator
/// 
/// Bug: Compiler fails to add & or .clone() for nested field access like stack.item.id
/// where stack is from &collection and parameter expects &String
///
/// Real-world case from windjammer-game/rpg/trading.wj:
/// ```windjammer
/// for stack in &self.merchant_offer {
///     if !merchant.has_item(stack.item.id, stack.quantity) {
///         return false
///     }
/// }
/// ```

use std::fs;
use std::process::Command;

#[test]
fn test_nested_field_to_borrowed_param() {
    let source = r#"
struct Item {
    id: string
}

struct Stack {
    item: Item
    quantity: i32
}

struct Merchant {
    stacks: Vec<Stack>
}

impl Merchant {
    fn has_item(item_id: string, quantity: i32) -> bool {
        true
    }
    
    fn check_offer(offers: Vec<Stack>) -> bool {
        for stack in &offers {
            if !has_item(stack.item.id, stack.quantity) {
                return false
            }
        }
        true
    }
}

fn main() {}
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
    
    let output = Command::new("wj")
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
    
    // Compile with rustc
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
    
    // Verify: Should generate &stack.item.id (not stack.item.id)
    // Because has_item(item_id: string) → has_item(item_id: &String)
    // And stack.item.id is accessed from borrowed stack
    assert!(
        generated.contains("&stack.item.id") || generated.contains("stack.item.id.clone()"),
        "Should add & or .clone() for nested field: Expected '&stack.item.id' or 'stack.item.id.clone()'"
    );
    
    fs::remove_dir_all(&test_dir).ok();
}
