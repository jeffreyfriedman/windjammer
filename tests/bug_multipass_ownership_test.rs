/// TDD Test: Multi-Pass Ownership Inference
/// 
/// The Proper Solution: Iterate ownership analysis until convergence
/// 
/// Problem: Single-pass analysis can't infer correct ownership for pass-through parameters
/// because the callee's signature doesn't exist yet.
///
/// Example:
/// ```windjammer
/// fn has_item(id: string) -> bool { true }  // Inferred: &String (Borrowed)
/// fn wrapper(item_id: string) -> bool {      // Should: &String (passes to has_item)
///     has_item(item_id)                     // Actual: String (conservative guess)
/// }
/// ```
///
/// Solution: Multi-pass analysis
/// Pass 1: Conservative inference → Build initial registry
/// Pass 2: Re-infer using registry → Update signatures
/// Pass N: Iterate until convergence (no changes)

use std::fs;
use std::process::Command;

#[test]
fn test_passthrough_borrowed_convergence() {
    // Simplest case: wrapper passes parameter to function expecting Borrowed
    let source = r#"
fn leaf_fn(id: string) -> bool {
    // Only reads id (comparison)
    id == "test"
}

fn wrapper_fn(item_id: string) -> bool {
    // Only passes item_id to leaf_fn (which wants &String)
    leaf_fn(item_id)
}

fn main() {
    let id = "sword"
    let result = wrapper_fn(id)
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
    
    // Verify both functions infer Borrowed
    assert!(
        generated.contains("fn leaf_fn(id: &String)") || generated.contains("fn leaf_fn(_id: &String)"),
        "leaf_fn should have Borrowed param (&String)"
    );
    assert!(
        generated.contains("fn wrapper_fn(item_id: &String)") || generated.contains("fn wrapper_fn(_item_id: &String)"),
        "wrapper_fn should have Borrowed param (&String) after multi-pass convergence"
    );
    
    fs::remove_dir_all(&test_dir).ok();
}

#[test]
fn test_method_passthrough_convergence() {
    // Real-world case from trading.wj: Merchant::has_item → Inventory::has_item
    let source = r#"
struct Inventory {
    items: Vec<string>
}

impl Inventory {
    fn has(id: string) -> bool {
        for item in &self.items {
            if item == id {
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
    fn has_item(item_id: string) -> bool {
        self.inventory.has(item_id)
    }
}

fn check(merchant: Merchant) -> bool {
    // Called with borrowed string
    merchant.has_item("sword")
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
    
    // Verify multi-pass convergence
    assert!(
        generated.contains("fn has(&self, id: &String)") || generated.contains("fn has(&self, _id: &String)"),
        "Inventory::has should have Borrowed param"
    );
    assert!(
        generated.contains("fn has_item(&self, item_id: &String)") || generated.contains("fn has_item(&self, _item_id: &String)"),
        "Merchant::has_item should infer Borrowed from Inventory::has"
    );
    
    fs::remove_dir_all(&test_dir).ok();
}

#[test]
fn test_circular_dependency_convergence() {
    // Edge case: mutual recursion should converge
    let source = r#"
fn foo(x: string) -> bool {
    if x == "stop" {
        true
    } else {
        bar(x)
    }
}

fn bar(y: string) -> bool {
    if y == "stop" {
        false
    } else {
        foo(y)
    }
}

fn main() {
    let result = foo("test")
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
    
    // Both should converge to Borrowed (only used in comparisons and pass-through)
    assert!(
        generated.contains("fn foo(x: &String)") || generated.contains("fn foo(_x: &String)"),
        "foo should have Borrowed param after convergence"
    );
    assert!(
        generated.contains("fn bar(y: &String)") || generated.contains("fn bar(_y: &String)"),
        "bar should have Borrowed param after convergence"
    );
    
    fs::remove_dir_all(&test_dir).ok();
}
