#[derive(Clone, Debug, PartialEq)]
enum DialogCondition {
    HasItem(String, i32),
}

impl DialogCondition {
#[inline]
pub fn evaluate(self, gs: GameState) -> bool {
        match self {
            DialogCondition::HasItem(item_id, qty) => {
                gs.inventory.has_item(item_id.clone(), qty.clone())
            },
        }
}
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
struct GameState {
    inventory: Inventory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
struct Inventory;

impl Inventory {
#[inline]
pub fn has_item(&self, item_id: &str, _min_qty: i32) -> bool {
        false
}
}

