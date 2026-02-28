/// TDD Test: Nested Field Access Through Borrowed Iterator
/// 
/// Bug: Compiler fails to handle nested field access like stack.item.id
/// where stack is from &collection, needs to generate &stack.item.id or stack.item.id.clone()
/// depending on parameter ownership.
///
/// Example:
/// ```windjammer
/// for stack in &stacks {
///     has_item(stack.item.id)  // stack.item.id is nested field access
/// }
/// ```

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_nested_field_borrowed_param() {
    // Windjammer source with nested field access
    let source = r#"
struct Item {
    id: string
    name: string
}

struct Stack {
    item: Item
    quantity: i32
}

fn has_item(item_id: string, quantity: i32) -> bool {
    true
}

fn check_stacks(stacks: Vec<Stack>) -> bool {
    for stack in &stacks {
        if !has_item(stack.item.id, stack.quantity) {
            return false
        }
    }
    true
}

fn main() {
    let stacks = Vec::new()
    let result = check_stacks(stacks)
}
"#;

    // Generate Rust code
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
    
    // Compile with wj
    let output = Command::new("wj")
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .output()
        .expect("Failed to run wj compiler");
    
    // Read generated Rust
    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file)
        .expect("Failed to read generated Rust file");
    
    println!("Generated Rust code:\n{}", generated);
    
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
        panic!("Rustc compilation failed:\n{}\n\nGenerated code:\n{}", stderr, generated);
    }
    
    // Verify generated code has correct ownership handling
    // Should generate: has_item(&stack.item.id, stack.quantity)
    // NOT: has_item(stack.item.id, stack.quantity) ← would cause E0507
    assert!(
        generated.contains("&stack.item.id") || generated.contains("stack.item.id.clone()"),
        "Should add & or .clone() for nested field access through borrowed iterator"
    );
    
    // Clean up
    fs::remove_dir_all(&test_dir).ok();
}

#[test]
fn test_nested_field_owned_param() {
    // Windjammer source where parameter expects owned String
    let source = r#"
struct Item {
    id: string
}

struct Stack {
    item: Item
}

fn remove_item(item_id: string) {
    // Takes owned String
}

fn process(stacks: Vec<Stack>) {
    for stack in &stacks {
        remove_item(stack.item.id)  // Should clone for owned param
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
    
    println!("Generated Rust code:\n{}", generated);
    
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
        panic!("Rustc compilation failed:\n{}\n\nGenerated code:\n{}", stderr, generated);
    }
    
    // Should generate: remove_item(stack.item.id.clone())
    // Because remove_item takes owned String, not &String
    assert!(
        generated.contains("stack.item.id.clone()"),
        "Should add .clone() for nested field when parameter expects owned"
    );
    
    fs::remove_dir_all(&test_dir).ok();
}
