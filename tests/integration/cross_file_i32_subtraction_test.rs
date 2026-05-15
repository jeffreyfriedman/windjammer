//! TDD: Cross-file i32 subtraction should not insert `as u32` casts.
//! Reproduces a bug where library compilation mode incorrectly casts
//! i32 struct field operands to u32 in subtraction expressions.

use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_cross_file_i32_field_subtraction_no_u32_cast() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    let build = dir.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    // File 1: Define Item and ItemStack with i32 fields (rpg module)
    let rpg = src.join("rpg");
    std::fs::create_dir_all(&rpg).unwrap();
    std::fs::write(
        rpg.join("item.wj"),
        r#"
pub struct Item {
    pub id: string,
    pub max_stack: i32,
}

pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

impl ItemStack {
    pub fn new(item: Item) -> ItemStack {
        ItemStack { item: item, quantity: 1 }
    }
}
"#,
    )
    .unwrap();

    // File 2: Define a DIFFERENT ItemStack with u32 fields (inventory module)
    // This creates the collision: two "ItemStack" structs with different field types
    let inv = src.join("inventory");
    std::fs::create_dir_all(&inv).unwrap();
    std::fs::write(
        inv.join("item_stack.wj"),
        r#"
pub struct InvItem {
    pub id: u32,
    pub max_stack: u32,
}

pub struct ItemStack {
    pub item: InvItem,
    pub quantity: u32,
}
"#,
    )
    .unwrap();

    // File 3: Use rpg::Item/ItemStack in subtraction, comparison, AND nested pattern matching
    std::fs::write(
        rpg.join("inventory.wj"),
        r#"
use crate::rpg::item::Item
use crate::rpg::item::ItemStack

pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
    pub capacity: i32,
}

impl Inventory {
    pub fn compute_space(self, slot: i32) -> i32 {
        if let Some(stack) = self.slots[slot as usize] {
            let can_add = stack.item.max_stack - stack.quantity
            return can_add
        }
        0
    }

    pub fn remove_item(self, slot: i32, quantity: i32) -> bool {
        if let Some(stack) = self.slots[slot as usize] {
            if quantity >= stack.quantity {
                self.slots[slot as usize] = None
                return true
            }
        }
        false
    }

    pub fn remove_by_id(self, item_id: string, quantity: i32) -> bool {
        let mut remaining: i32 = quantity
        let mut i: i32 = 0
        while i < self.capacity && remaining > 0 {
            if let Some(stack) = self.slots[i as usize] {
                if stack.quantity <= remaining {
                    remaining = remaining - stack.quantity
                    self.slots[i as usize] = None
                }
            }
            i = i + 1
        }
        remaining == 0
    }

    pub fn move_item(self, from_slot: i32, to_slot: i32) -> bool {
        let from_data = if let Some(from_stack) = self.slots[from_slot as usize] {
            Some((from_stack.item.id.clone(), from_stack.item.stackable, from_stack.quantity))
        } else {
            None
        }
        if let Some((from_item_id, _from_stackable, from_quantity)) = from_data {
            if let Some(to_stack) = self.slots[to_slot as usize] {
                if from_item_id == to_stack.item.id && to_stack.item.stackable {
                    let can_add = to_stack.item.max_stack - to_stack.quantity
                    if can_add >= from_quantity {
                        return true
                    }
                }
            }
        }
        false
    }
}
"#,
    )
    .unwrap();

    build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[])
        .expect("Library build failed");

    // Check the rpg/inventory.rs output (may be flat or nested structure)
    let inventory_rs = std::fs::read_to_string(build.join("rpg").join("inventory.rs"))
        .or_else(|_| std::fs::read_to_string(build.join("inventory.rs")))
        .expect("Could not read generated inventory.rs");

    eprintln!("=== GENERATED inventory.rs ===\n{}", inventory_rs);

    assert!(
        !inventory_rs.contains("as u32"),
        "rpg::inventory should use i32 fields from rpg::item::ItemStack, not u32 from inventory::item_stack::ItemStack.\nGenerated:\n{}",
        inventory_rs
    );
}
