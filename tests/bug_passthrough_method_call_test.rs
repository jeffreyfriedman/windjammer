/// TDD test: Passthrough inference for method calls on self.field
///
/// Bug: `self.inventory.add_item(item, quantity)` in `Merchant::add_item`
/// doesn't trigger passthrough inference to keep `item` as Owned.
///
/// The `item` parameter is passed directly to `Inventory::add_item`
/// which stores it (Owned). The passthrough should propagate this.

use std::process::Command;
use std::fs;

fn transpile_wj(source: &str) -> String {
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
    fs::read_to_string(&rust_file)
        .expect("Failed to read generated Rust file")
}

#[test]
fn test_passthrough_to_self_field_method() {
    let source = r#"
pub struct Item {
    pub name: String,
    pub weight: f32,
}

pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

impl ItemStack {
    pub fn new(item: Item, quantity: i32) -> ItemStack {
        ItemStack { item, quantity }
    }
}

pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
}

impl Inventory {
    pub fn add_item(self, item: Item, quantity: i32) {
        self.slots[0] = Some(ItemStack::new(item, quantity))
    }
}

pub struct Merchant {
    pub inventory: Inventory,
}

impl Merchant {
    pub fn add_item(self, item: Item, quantity: i32) {
        self.inventory.add_item(item, quantity)
    }
}
"#;

    let generated = transpile_wj(source);
    println!("Generated:\n{}", generated);

    // Merchant::add_item should have item: Item (Owned) via passthrough
    // from Inventory::add_item which stores the item
    assert!(
        !generated.contains("impl Merchant")
            || !generated.contains("fn add_item(&mut self, item: &Item"),
        "Merchant::add_item item param should be Owned via passthrough. Got:\n{}",
        generated
    );
}
