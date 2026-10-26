use windjammer_runtime::test::*;
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct Item {
    pub id: String,
    pub stackable: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct ItemStack {
    pub item: Item,
    pub quantity: u32,
}

impl ItemStack {
#[inline]
pub fn add(&mut self, amount: u32) -> bool {
        self.quantity += amount;
        true
}
}

#[test]

fn test_i32_to_u32_parameter() {
    let item = Item { id: String::from("test"), stackable: true };
    let mut stack = ItemStack { item, quantity: 5_u32 };
    let quantity: i32 = 10_i32;
    let q: i32 = quantity;
    stack.add((q as u32));
}

