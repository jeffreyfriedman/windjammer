#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

#[path = "common/test_utils.rs"]
mod test_utils;

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
#[test]
fn test_index_assign_with_some_constructor() {
    let source = r#"
pub struct Item {
    pub name: string,
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

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    // item should be Owned because it's stored via index assignment
    assert!(
        generated.contains("fn add_item(&mut self, item: Item"),
        "Parameter stored via index assignment should be Owned. Got:\n{}",
        generated
    );
}
