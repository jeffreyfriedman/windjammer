/// Test: Cross-module method signatures should be available during compilation
///
/// Bug #8: Transpiler doesn't load method signatures from imported modules
///
/// When a type is imported from another module, the transpiler falls back to
/// heuristics for method calls, incorrectly adding `&` to Copy type arguments.
///
/// Example:
/// ```
/// // item.wj
/// struct Stack {
///     quantity: i32,
/// }
/// impl Stack {
///     pub fn remove(&mut self, amount: i32) -> bool { ... }
/// }
///
/// // inventory.wj
/// use crate::Stack
///
/// fn test(stack: &mut Stack, quantity: i32) {
///     stack.remove(quantity);  // Incorrectly generates: remove(&quantity)
/// }
/// ```
///
/// Root cause: The two-pass compilation doesn't cache signatures across modules,
/// so when compiling inventory.wj, it doesn't know Stack::remove takes i32 by value.
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_cross_module_copy_type_arg() {
    let temp_dir = TempDir::new().unwrap();

    // Create module 1: defines Stack with remove method
    let item_source = r#"
pub struct Stack {
    pub quantity: i32,
}

impl Stack {
    pub fn remove(&mut self, amount: i32) -> bool {
        if self.quantity < amount {
            return false
        }
        self.quantity = self.quantity - amount
        true
    }
}
"#;

    // Create module 2: imports and uses Stack
    let inventory_source = r#"
use crate::Stack

pub fn remove_from_stack(stack: &mut Stack, quantity: i32) {
    stack.remove(quantity);
}
"#;

    // Write both files
    fs::create_dir_all(temp_dir.path().join("src")).unwrap();
    fs::write(temp_dir.path().join("src").join("item.wj"), item_source).unwrap();
    fs::write(
        temp_dir.path().join("src").join("inventory.wj"),
        inventory_source,
    )
    .unwrap();

    // Create wj.toml
    let toml_content = r#"
[package]
name = "test_cross_module"
version = "0.1.0"

[lib]
path = "src/item.wj"

[[bin]]
name = "inventory"
path = "src/inventory.wj"
"#;
    fs::write(temp_dir.path().join("wj.toml"), toml_content).unwrap();

    // Compile all files together (simulating multi-file project compilation)
    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg("src/") // Compile all files in src/ directory together
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    if !wj_output.status.success() {
        panic!(
            "Compilation failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&wj_output.stdout),
            String::from_utf8_lossy(&wj_output.stderr)
        );
    }

    // Check generated inventory code
    let inventory_rs = temp_dir.path().join("build").join("inventory.rs");
    let generated = fs::read_to_string(&inventory_rs).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated inventory.rs. Build output:\n{}",
            String::from_utf8_lossy(&wj_output.stdout)
        )
    });

    // Should NOT add & to Copy type argument
    assert!(
        generated.contains("stack.remove(quantity)"),
        "Expected 'stack.remove(quantity)' but got:\n{}",
        generated
    );

    assert!(
        !generated.contains("stack.remove(&quantity)"),
        "Should NOT generate 'stack.remove(&quantity)', found in:\n{}",
        generated
    );
}

#[test]
fn test_cross_module_multiple_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create module 1: Stack type
    let stack_source = r#"
pub struct Stack {
    pub quantity: i32,
}

impl Stack {
    pub fn new(quantity: i32) -> Stack {
        Stack { quantity: quantity }
    }
    
    pub fn remove(&mut self, amount: i32) -> bool {
        if self.quantity < amount {
            return false
        }
        self.quantity = self.quantity - amount
        true
    }
    
    pub fn add(&mut self, amount: i32) {
        self.quantity = self.quantity + amount
    }
}
"#;

    // Create module 2: uses Stack
    let inventory_source = r#"
use crate::Stack

pub struct Inventory {
    pub items: Vec<Option<Stack>>,
}

impl Inventory {
    pub fn new() -> Inventory {
        Inventory { items: Vec::new() }
    }
    
    pub fn remove_at(&mut self, index: i32, quantity: i32) {
        if let Some(stack) = &mut self.items[index as usize] {
            stack.remove(quantity);
        }
    }
    
    pub fn add_at(&mut self, index: i32, quantity: i32) {
        if let Some(stack) = &mut self.items[index as usize] {
            stack.add(quantity);
        }
    }
}
"#;

    // Write files
    fs::create_dir_all(temp_dir.path().join("src")).unwrap();
    fs::write(temp_dir.path().join("src").join("stack.wj"), stack_source).unwrap();
    fs::write(
        temp_dir.path().join("src").join("inventory.wj"),
        inventory_source,
    )
    .unwrap();

    // Create wj.toml
    let toml_content = r#"
[package]
name = "test_multi_file"
version = "0.1.0"
"#;
    fs::write(temp_dir.path().join("wj.toml"), toml_content).unwrap();

    // Compile all files
    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg("src/")
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    if !wj_output.status.success() {
        panic!(
            "Compilation failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&wj_output.stdout),
            String::from_utf8_lossy(&wj_output.stderr)
        );
    }

    // Check generated inventory code
    let inventory_rs = temp_dir.path().join("build").join("inventory.rs");
    let generated = fs::read_to_string(&inventory_rs)
        .unwrap_or_else(|_| panic!("Failed to read generated inventory.rs"));

    // Both methods should use correct signatures
    assert!(
        generated.contains("stack.remove(quantity)"),
        "Expected 'stack.remove(quantity)' but got:\n{}",
        generated
    );

    assert!(
        generated.contains("stack.add(quantity)"),
        "Expected 'stack.add(quantity)' but got:\n{}",
        generated
    );

    assert!(
        !generated.contains("stack.remove(&quantity)"),
        "Should NOT generate 'stack.remove(&quantity)', found in:\n{}",
        generated
    );

    assert!(
        !generated.contains("stack.add(&quantity)"),
        "Should NOT generate 'stack.add(&quantity)', found in:\n{}",
        generated
    );
}

#[test]
fn test_cross_module_nested_if_let() {
    let temp_dir = TempDir::new().unwrap();

    // Create module with type
    let item_source = r#"
pub struct Item {
    pub id: string,
}

pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

impl ItemStack {
    pub fn remove(&mut self, amount: i32) -> bool {
        if self.quantity < amount {
            return false
        }
        self.quantity = self.quantity - amount
        true
    }
}
"#;

    // Create module that uses type
    let inventory_source = r#"
use crate::ItemStack

pub fn process(slots: &mut Vec<Option<ItemStack>>, index: i32, quantity: i32) {
    if let Some(stack) = &mut slots[index as usize] {
        stack.remove(quantity);
    }
}
"#;

    // Write files
    fs::create_dir_all(temp_dir.path().join("src")).unwrap();
    fs::write(temp_dir.path().join("src").join("item.wj"), item_source).unwrap();
    fs::write(
        temp_dir.path().join("src").join("inventory.wj"),
        inventory_source,
    )
    .unwrap();

    // Create wj.toml
    let toml_content = r#"
[package]
name = "test_nested"
version = "0.1.0"
"#;
    fs::write(temp_dir.path().join("wj.toml"), toml_content).unwrap();

    // Compile
    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg("src/")
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    if !wj_output.status.success() {
        panic!(
            "Compilation failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&wj_output.stdout),
            String::from_utf8_lossy(&wj_output.stderr)
        );
    }

    // Check generated code
    let inventory_rs = temp_dir.path().join("build").join("inventory.rs");
    let generated = fs::read_to_string(&inventory_rs)
        .unwrap_or_else(|_| panic!("Failed to read generated inventory.rs"));

    // Should work correctly even in nested context
    assert!(
        generated.contains("stack.remove(quantity)"),
        "Expected 'stack.remove(quantity)' but got:\n{}",
        generated
    );

    assert!(
        !generated.contains("stack.remove(&quantity)"),
        "Should NOT generate 'stack.remove(&quantity)', found in:\n{}",
        generated
    );
}
