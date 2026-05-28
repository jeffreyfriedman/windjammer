#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_auto_clone_parameter_moved_in_simple_loop() {
    let source = r#"
    struct Item {
        name: String,
        max_stack: u32,
    }
    impl Item {
        fn max_stack(self) -> u32 {
            self.max_stack
        }
    }
    struct ItemStack {
        item: Item,
        quantity: u32,
    }
    impl ItemStack {
        fn new(item: Item, quantity: u32) -> ItemStack {
            ItemStack { item: item, quantity: quantity }
        }
    }
    struct Inventory {
        slots: Vec<Option<ItemStack>>,
    }
    impl Inventory {
        fn add_items(self, item: Item, quantity: u32) {
            let mut remaining = quantity
            while remaining > 0 {
                let to_add = remaining.min(item.max_stack())
                self.slots.push(Some(ItemStack::new(item, to_add)))
                remaining = remaining - to_add
            }
        }
    }
    "#;
    let output = test_utils::compile_single(source);
    eprintln!("Generated:\n{}", output);

    let has_clone = output.contains("item.clone()");
    assert!(
        has_clone,
        "Parameter 'item' moved inside a while loop should be auto-cloned to avoid E0382.\nGenerated:\n{}",
        output
    );
}

#[test]
fn test_auto_clone_parameter_moved_in_complex_loop() {
    let source = r#"
    struct Item {
        name: String,
        max_stack: u32,
    }
    impl Item {
        fn max_stack(self) -> u32 {
            self.max_stack
        }
        fn is_stackable(self) -> bool {
            true
        }
        fn id(self) -> u32 {
            0
        }
    }
    struct ItemStack {
        item: Item,
        quantity: u32,
    }
    impl ItemStack {
        fn new(item: Item, quantity: u32) -> ItemStack {
            ItemStack { item: item, quantity: quantity }
        }
    }
    struct Inventory {
        slots: Vec<Option<ItemStack>>,
        capacity: usize,
    }
    impl Inventory {
        fn add_item(self, item: Item, quantity: u32) -> bool {
            let item_id = item.id()
            let mut remaining = quantity

            if item.is_stackable() {
                for i in 0..self.slots.len() {
                    if remaining == 0 {
                        return true
                    }
                }
            }

            while remaining > 0 {
                let mut found_empty = false
                let mut empty_index: usize = 0
                for j in 0..self.slots.len() {
                    if self.slots[j].is_none() {
                        found_empty = true
                        empty_index = j
                        break
                    }
                }

                if found_empty {
                    let to_add = remaining.min(item.max_stack())
                    self.slots[empty_index] = Some(ItemStack::new(item, to_add))
                    remaining = remaining - to_add
                } else {
                    break
                }
            }

            remaining == 0
        }
    }
    "#;
    let output = test_utils::compile_single(source);
    eprintln!("Generated:\n{}", output);

    let has_clone = output.contains("item.clone()");
    assert!(
        has_clone,
        "Parameter 'item' moved inside a complex while loop (with nested for/if) should be auto-cloned.\nGenerated:\n{}",
        output
    );
}

#[test]
fn test_auto_clone_partial_move_field_access() {
    let source = r#"
    struct Item {
        id: String,
        weight: f32,
    }
    struct ItemStack {
        item: Item,
        quantity: i32,
    }
    impl ItemStack {
        fn new_with_quantity(item: Item, quantity: i32) -> ItemStack {
            ItemStack { item: item, quantity: quantity }
        }
        fn remove(self, quantity: i32) {
            self.quantity = self.quantity - quantity
        }
    }
    fn split_stack(s: ItemStack, quantity: i32) -> ItemStack {
        let result = ItemStack::new_with_quantity(s.item, quantity)
        s.remove(quantity)
        result
    }
    "#;
    let output = test_utils::compile_single(source);
    eprintln!("Generated:\n{}", output);

    let has_clone = output.contains("s.item.clone()");
    assert!(
        has_clone,
        "Field 's.item' moved while parent 's' is used later should be auto-cloned.\nGenerated:\n{}",
        output
    );
}

#[test]
fn test_auto_clone_partial_move_in_match_arm() {
    let source = r#"
    struct Item {
        id: String,
        weight: f32,
    }
    struct ItemStack {
        item: Item,
        quantity: i32,
    }
    impl ItemStack {
        fn new_with_quantity(item: Item, quantity: i32) -> ItemStack {
            ItemStack { item: item, quantity: quantity }
        }
        fn remove(self, quantity: i32) {
            self.quantity = self.quantity - quantity
        }
    }
    struct Inventory {
        slots: Vec<Option<ItemStack>>,
    }
    impl Inventory {
        fn remove_partial(self, slot_index: i32, quantity: i32) -> Option<ItemStack> {
            let stack = self.slots[slot_index as usize]
            match stack {
                Some(mut s) => {
                    let result = ItemStack::new_with_quantity(s.item, quantity)
                    s.remove(quantity)
                    self.slots[slot_index as usize] = Some(s)
                    Some(result)
                },
                None => None,
            }
        }
    }
    "#;
    let output = test_utils::compile_single(source);
    eprintln!("Generated:\n{}", output);

    let has_clone = output.contains("s.item.clone()");
    assert!(
        has_clone,
        "Field 's.item' moved inside match arm while parent 's' is used later should be auto-cloned.\nGenerated:\n{}",
        output
    );
}
