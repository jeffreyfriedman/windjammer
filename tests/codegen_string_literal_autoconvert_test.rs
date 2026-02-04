use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

/// Test that string literals are automatically converted to String in function calls
///
/// Bug: Passing "literal" to fn(String) requires .to_string(), but shouldn't
/// Fix: Compiler should auto-convert &str literals to String when needed
#[test]
fn test_string_literal_to_string_in_function_call() {
    let source = r#"
struct Item {
    pub id: string,
    pub name: string,
    pub description: string,
}

impl Item {
    pub fn new(id: string, name: string, description: string) -> Item {
        Item { id: id, name: name, description: description }
    }
}

fn main() {
    // Should auto-convert string literals to String
    let item = Item::new("sword", "Iron Sword", "A basic sword")
    assert_eq(item.id, "sword")
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("test.wj");
    fs::write(&input_path, source).unwrap();

    let output = Command::new(get_wj_compiler())
        .args(["build", input_path.to_str().unwrap(), "--no-cargo"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        panic!(
            "Windjammer compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let rust_path = temp_dir.path().join("build/test.rs");
    let rust_code = fs::read_to_string(&rust_path).unwrap();

    println!("Generated Rust:\n{}", rust_code);

    // Verify .to_string() was added automatically
    assert!(
        rust_code.contains(r#"Item::new("sword".to_string(), "Iron Sword".to_string(), "A basic sword".to_string())"#),
        "Expected automatic .to_string() conversion in function call"
    );
}

#[test]
fn test_string_literal_in_method_call() {
    let source = r#"
struct Manager {
    pub items: Vec<string>,
}

impl Manager {
    pub fn new() -> Manager {
        Manager { items: Vec::new() }
    }
    
    pub fn add(&mut self, item: string) {
        self.items.push(item)
    }
}

fn main() {
    let mut mgr = Manager::new()
    mgr.add("test")  // Should auto-convert
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("test.wj");
    fs::write(&input_path, source).unwrap();

    let output = Command::new(get_wj_compiler())
        .args(["build", input_path.to_str().unwrap(), "--no-cargo"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        panic!(
            "Windjammer compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let rust_path = temp_dir.path().join("build/test.rs");
    let rust_code = fs::read_to_string(&rust_path).unwrap();

    // Verify .to_string() was added
    assert!(
        rust_code.contains(r#"mgr.add("test".to_string())"#),
        "Expected automatic .to_string() conversion in method call"
    );
}
