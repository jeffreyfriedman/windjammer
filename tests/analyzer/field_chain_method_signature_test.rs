/// TDD Test: Field-chain method signature resolution
///
/// Tests that game_state.inventory.has_item(item_id) correctly resolves:
/// 1. game_state → GameState type
/// 2. inventory field → Inventory type
/// 3. has_item method → Inventory::has_item signature
/// 4. item_id parameter → adds & for String → &str conversion
///
/// This validates Phase 3: Field-chain type resolution works end-to-end
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_field_chain_single_level() {
    // Test: obj.inventory.has_item(id) where inventory is a field of obj
    let code = r#"
pub struct Inventory {
    pub items: Vec<string>,
}

impl Inventory {
    pub fn has_item(self, item_id: string) -> bool {
        self.items.contains(item_id)
    }
}

pub struct GameState {
    pub inventory: Inventory,
}

pub fn check_item(state: GameState, id: string) -> bool {
    state.inventory.has_item(id)  // Should add & to id
}

pub fn main() {
    let state = GameState { inventory: Inventory { items: Vec::new() } }
    let result = check_item(state, "sword")
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Field-chain method signature resolution should work:\n{:?}",
        result.err()
    );
}

#[test]
fn test_match_arm_with_field_chain() {
    // Test: Match arm binding used with field-chain method call
    let code = r#"
pub struct Inventory {
    pub items: Vec<string>,
}

impl Inventory {
    pub fn has_item(self, item_id: string, qty: i32) -> bool {
        self.items.contains(item_id)
    }
}

pub struct GameState {
    pub inventory: Inventory,
}

pub enum Condition {
    HasItem(string, i32),
}

impl Condition {
    pub fn evaluate(self, state: GameState) -> bool {
        match self {
            Condition::HasItem(item_id, qty) => {
                state.inventory.has_item(item_id, qty)  // Should add & to item_id
            }
        }
    }
}

pub fn main() {
    let state = GameState { inventory: Inventory { items: Vec::new() } }
    let cond = Condition::HasItem("sword", 1)
    let result = cond.evaluate(state)
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Match arm with field-chain should work:\n{:?}",
        result.err()
    );
}

#[test]
fn test_deep_field_chain() {
    // Test: Deeper nesting (a.b.c.method(x))
    let code = r#"
pub struct Player {
    pub name: string,
}

impl Player {
    pub fn has_name(self, n: string) -> bool {
        self.name == n
    }
}

pub struct GameData {
    pub player: Player,
}

pub struct GameState {
    pub data: GameData,
}

pub fn check_player(state: GameState, name: string) -> bool {
    state.data.player.has_name(name)  // Should add & to name
}

pub fn main() {
    let state = GameState { 
        data: GameData { 
            player: Player { name: "Alice" } 
        } 
    }
    let result = check_player(state, "Alice")
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Deep field-chain should work:\n{:?}",
        result.err()
    );
}
