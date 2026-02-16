/// TDD Test: No double .clone().clone() when source already has explicit .clone()
///
/// Bug: When Windjammer source has `stack.item.clone()` (explicit clone because
/// Item is non-Copy), the auto-clone system adds another .clone() because it sees
/// `stack.item` used multiple times. Result: `stack.item.clone().clone()`.
///
/// Root Cause: The auto-clone on FieldAccess adds .clone() to `stack.item`,
/// and then the explicit .clone() MethodCall from the source generates a second one.
///
/// Expected: Only one .clone() should appear.

use std::process::Command;
use std::io::Write;

fn compile_wj(source: &str) -> String {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let wj_path = dir.path().join("test.wj");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();

    let mut file = std::fs::File::create(&wj_path).unwrap();
    file.write_all(source.as_bytes()).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "--no-cargo",
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run wj");

    let rs_path = out_dir.join("test.rs");
    if rs_path.exists() {
        std::fs::read_to_string(&rs_path).unwrap()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "No output file generated.\nstdout: {}\nstderr: {}",
            stdout, stderr
        );
    }
}

#[test]
fn test_no_double_clone_on_explicit_clone() {
    let source = r#"
pub struct Item {
    pub id: String,
    pub name: String,
}

pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

pub struct Inventory {
    pub items: Vec<Item>,
}

impl Inventory {
    pub fn add_item(self, item: Item) {
        self.items.push(item)
    }
}

pub struct Trade {
    pub offer: Vec<ItemStack>,
}

impl Trade {
    pub fn execute(self, inv: Inventory) {
        for stack in &self.offer {
            inv.add_item(stack.item.clone())
        }
    }
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // Should have exactly one .clone(), NOT .clone().clone()
    assert!(
        !generated.contains(".clone().clone()"),
        "Should not have double .clone().clone().\nGenerated:\n{}",
        generated
    );

    // Should still have the single clone (Item is non-Copy)
    assert!(
        generated.contains("stack.item.clone()"),
        "Should have single .clone() for non-Copy field.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_no_double_clone_field_used_multiple_times() {
    // Field used multiple times: once with .clone(), once without
    let source = r#"
pub struct Item {
    pub id: String,
    pub name: String,
}

pub struct Container {
    pub items: Vec<Item>,
}

impl Container {
    pub fn process(self) {
        for item in &self.items {
            let id = item.id.clone()
            let name = item.name.clone()
        }
    }
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        !generated.contains(".clone().clone()"),
        "Should not have double .clone().clone().\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_no_double_clone_multi_use_in_loop() {
    // Mimics the game code pattern: stack.item used multiple times in same loop,
    // some with explicit .clone(), some for .id.clone()
    let source = r#"
pub struct Item {
    pub id: String,
    pub name: String,
}

pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

pub struct Inventory {
    pub items: Vec<Item>,
}

impl Inventory {
    pub fn add_item(self, item: Item) {
        self.items.push(item)
    }

    pub fn remove_item_by_id(self, id: String, qty: i32) -> bool {
        true
    }
}

pub struct Trade {
    pub offer: Vec<ItemStack>,
}

impl Trade {
    pub fn execute(self, inv: Inventory) {
        for stack in &self.offer {
            inv.remove_item_by_id(stack.item.id.clone(), stack.quantity)
            inv.add_item(stack.item.clone())
        }
    }
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // Should NOT have .clone().clone() anywhere
    assert!(
        !generated.contains(".clone().clone()"),
        "Should not have double .clone().clone().\nGenerated:\n{}",
        generated
    );

    // Should have single .clone() for non-Copy field
    assert!(
        generated.contains("stack.item.clone()"),
        "Should have single .clone() for stack.item.\nGenerated:\n{}",
        generated
    );
}
