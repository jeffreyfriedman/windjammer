/// TDD Test: Ownership Inference Pass-Through Bug
/// 
/// Bug: Analyzer incorrectly infers Owned for parameters that are only
/// passed to callees expecting Borrowed parameters.
///
/// Example:
/// ```windjammer
/// fn wrapper(item_id: string) -> bool {
///     has_item(item_id)  // has_item expects &String
/// }
/// fn has_item(id: string) -> bool { true }
/// ```
/// 
/// Expected: wrapper should have item_id: &String (Borrowed)
/// Actual: wrapper has item_id: String (Owned) ❌
/// Result: Callers can't pass &String to wrapper!

use std::fs;
use std::process::Command;

#[test]
fn test_passthrough_to_borrowed_param() {
    let source = r#"
struct Inventory {
    items: Vec<string>
}

impl Inventory {
    fn has(id: string) -> bool {
        // Just a check, parameter not consumed
        for item_id in &self.items {
            if item_id == id {
                return true
            }
        }
        false
    }
}

struct Merchant {
    inventory: Inventory
}

impl Merchant {
    /// Wrapper that just calls inventory.has()
    /// Should infer: item_id: &String (Borrowed)
    /// Because Inventory::has expects &String (Borrowed)
    fn has_item(item_id: string) -> bool {
        self.inventory.has(item_id)
    }
    
    /// Check multiple items
    fn check_items(items: Vec<string>) -> bool {
        for id in &items {
            // id is &String from borrowed iterator
            // has_item should accept &String (Borrowed param)
            if !has_item(id) {
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
    
    // Verify: has_item should have &String parameter (Borrowed)
    // Because it only passes to Inventory::has which expects Borrowed
    assert!(
        generated.contains("fn has_item(&self, item_id: &String)") 
            || generated.contains("fn has_item(&self, _item_id: &String)"),
        "Expected has_item to have Borrowed parameter (&String), not Owned (String)"
    );
    
    // Verify: check_items should compile (calls has_item with borrowed iterator var)
    // If has_item incorrectly expects String, this would fail with E0308
    
    fs::remove_dir_all(&test_dir).ok();
}
