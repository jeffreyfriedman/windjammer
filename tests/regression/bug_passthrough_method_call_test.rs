#![cfg(not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
)))]

#[path = "../common/test_utils.rs"]
mod test_utils;

/// TDD test: Passthrough inference for method calls on self.field
///
/// Bug: `self.inventory.add_item(item, quantity)` in `Merchant::add_item`
/// doesn't trigger passthrough inference to keep `item` as Owned.
///
/// The `item` parameter is passed directly to `Inventory::add_item`
/// which stores it (Owned). The passthrough should propagate this.
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

    let generated = test_utils::compile_single(source);
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
