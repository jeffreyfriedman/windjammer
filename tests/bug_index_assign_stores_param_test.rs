use std::fs;
/// TDD test: Parameters stored via index assignment with nested constructors
///
/// Bug: `self.slots[i] = Some(ItemStack::new(item, qty))` doesn't detect
/// that `item` is stored because the `is_stored` check for index assignments
/// only matches direct identifiers, not nested expressions.
///
/// Root Cause: `Statement::Assignment { target: Index, value }` only checks
/// `matches!(value, Expression::Identifier { ... })`, missing nested storage.
///
/// Fix: Use `expression_stores_identifier` for index assignment values.
use std::process::Command;

fn transpile_wj(source: &str) -> String {
    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
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
    fs::read_to_string(&rust_file).expect("Failed to read generated Rust file")
}

#[test]
fn test_index_assign_with_some_constructor() {
    let source = r#"
pub struct Item {
    pub name: String,
    pub stackable: bool,
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
    pub capacity: i32,
}

impl Inventory {
    pub fn add_item(self, item: Item, quantity: i32) -> bool {
        if item.stackable {
            return false
        }
        let new_weight = item.weight * quantity as f32
        self.slots[0] = Some(ItemStack::new(item, quantity))
        true
    }
}
"#;

    let generated = transpile_wj(source);
    println!("Generated:\n{}", generated);

    // item should be Owned because it's stored via index assignment
    assert!(
        generated.contains("fn add_item(&mut self, item: Item"),
        "Parameter stored via index assignment should be Owned. Got:\n{}",
        generated
    );
}
