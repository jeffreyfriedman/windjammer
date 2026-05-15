#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

/// TDD Tests: Remaining dialog.wj compilation issues
///
/// This file documents the remaining 12 Rust compilation errors in dialog.wj
/// and provides TDD tests to reproduce and fix each category systematically.
///
/// CURRENT STATUS (2024-03-XX):
/// ✓ String→&str auto-conversion working for most cases (via heuristic fallback)
/// ✗ Vec<String>::push incorrectly adding & (5 E0308 errors)
/// ✗ self.field String→&str conversion not working (1 E0308)
/// ✗ Primitive deref in match patterns (1 E0614)
/// ✗ Option-returning method ownership inference (1 E0507)
/// ✗ For-loop tuple element mutability inference (5 E0594)
#[path = "../../common/test_utils.rs"]
mod test_utils;

// =============================================================================
// CATEGORY 1: Vec<String>::push Over-Borrowing (5× E0308)
// =============================================================================

#[test]
fn test_vec_push_should_not_add_ref() {
    let code = r#"
pub struct Node {
    pub actions: Vec<string>,
}

impl Node {
    pub fn add_action(self, action: string) {
        self.actions.push(action)  // Should NOT become push(&action)
    }
}

pub fn main() {
    let node = Node { actions: Vec::new() }
    node.add_action("test")
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Vec<String>::push should not add &:\n{:?}",
        result.err()
    );

    let rust_code = result.unwrap();
    assert!(
        rust_code.contains(".push(action)"),
        "Should not add & to Vec::push:\n{}",
        rust_code
    );
}

// =============================================================================
// CATEGORY 2: self.field String→&str Conversion (1× E0308)
// =============================================================================

#[test]
fn test_self_field_string_to_str_conversion() {
    let code = r#"
pub struct Checker {
    pub name_field: string,
}

pub struct Player {
    pub name: string,
}

impl Player {
    pub fn matches(self, name: string) -> bool {
        self.name == name
    }
}

pub struct StatCheck {
    pub stat_name: string,
    pub min_value: i32,
}

impl StatCheck {
    pub fn passes(self, player: Player) -> bool {
        player.matches(self.stat_name)  // Should add & for self.stat_name
    }
}

pub fn main() {
    let check = StatCheck { stat_name: "strength", min_value: 10 }
    let player = Player { name: "strength" }
    let result = check.passes(player)
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "self.field should auto-convert to &str:\n{:?}",
        result.err()
    );
}

// =============================================================================
// CATEGORY 3: Primitive Deref in Match Patterns (1× E0614)
// =============================================================================

#[test]
fn test_primitive_deref_in_match() {
    let code = r#"
pub enum Cost {
    Gold(i32),
}

impl Cost {
    pub fn amount(self) -> i32 {
        match self {
            Cost::Gold(amount) => {
                if *amount > 100 {  // Should NOT deref primitive
                    *amount
                } else {
                    0
                }
            }
        }
    }
}

pub fn main() {
    let cost = Cost::Gold(50)
    let amt = cost.amount()
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Should not deref primitive in match:\n{:?}",
        result.err()
    );

    let rust_code = result.unwrap();
    assert!(
        !rust_code.contains("*amount"),
        "Should not add * to primitive match binding:\n{}",
        rust_code
    );
}

// =============================================================================
// CATEGORY 4: Option-Returning Method Ownership (1× E0507)
// =============================================================================

#[test]
fn test_option_return_ownership() {
    let code = r#"
pub struct Node {
    pub id: i32,
}

pub struct Tree {
    pub nodes: Vec<Node>,
    pub current_id: i32,
}

impl Tree {
    pub fn get_current_node(self) -> Option<Node> {
        for node in self.nodes {
            if node.id == self.current_id {
                return Some(node)
            }
        }
        None
    }
    
    pub fn process(self) -> i32 {
        if let Some(node) = self.get_current_node() {  // E0507: cannot move out of *self
            node.id
        } else {
            0
        }
    }
}

pub fn main() {
    let tree = Tree { nodes: Vec::new(), current_id: 1 }
    let result = tree.process()
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Option-returning method should infer &self:\n{:?}",
        result.err()
    );
}

// =============================================================================
// CATEGORY 5: For-Loop Tuple Element Mutability (5× E0594)
// =============================================================================

#[test]
fn test_for_loop_tuple_mutability() {
    let code = r#"
pub struct Inventory {
    pub items: Vec<(string, i32)>,
}

impl Inventory {
    pub fn add_item(self, item_id: string, quantity: i32) {
        for (id, qty) in self.items {
            if *id == item_id {
                *qty = *qty + quantity  // E0594: cannot assign to *qty (behind & reference)
                return
            }
        }
        self.items.push((item_id, quantity))
    }
    
    pub fn remove_item(self, item_id: string, quantity: i32) -> bool {
        for (id, qty) in self.items {
            if *id == item_id {
                if *qty >= quantity {
                    *qty = *qty - quantity  // E0594: cannot assign to *qty
                    return true
                }
                return false
            }
        }
        false
    }
}

pub fn main() {
    let inv = Inventory { items: Vec::new() }
    inv.add_item("sword", 1)
    let success = inv.remove_item("sword", 1)
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "For-loop should infer &mut for tuple elements:\n{:?}",
        result.err()
    );
}

// =============================================================================
// INTEGRATION TEST: All Issues Together
// =============================================================================

#[test]
fn test_dialog_all_issues_fixed() {
    // This test uses a simplified version of the full dialog.wj pattern
    // It should compile successfully once all 5 categories are fixed
    let code = r#"
pub struct Player {
    pub gold: i32,
    pub attributes: Vec<(string, i32)>,
}

impl Player {
    pub fn get_attribute(self, name: string) -> i32 {
        for (attr, val) in self.attributes {
            if *attr == name {  // Category 3: Primitive deref
                return *val
            }
        }
        0
    }
    
    pub fn set_attribute(self, name: string, value: i32) {
        for (attr, val) in self.attributes {  // Category 5: Mutability
            if *attr == name {
                *val = value
                return
            }
        }
    }
}

pub struct StatCheck {
    pub stat_name: string,
    pub min_value: i32,
}

impl StatCheck {
    pub fn passes(self, player: Player) -> bool {
        player.get_attribute(self.stat_name) >= self.min_value  // Category 2: self.field
    }
}

pub struct Actions {
    pub list: Vec<string>,
}

impl Actions {
    pub fn add(self, action: string) {
        self.list.push(action)  // Category 1: Vec::push over-borrow
    }
}

pub struct Tree {
    pub nodes: Vec<i32>,
    pub current: i32,
}

impl Tree {
    pub fn get_current(self) -> Option<i32> {
        for node in self.nodes {
            if node == self.current {
                return Some(node)
            }
        }
        None
    }
    
    pub fn process(self) -> i32 {
        if let Some(node) = self.get_current() {  // Category 4: Option ownership
            node
        } else {
            0
        }
    }
}

pub fn main() {
    let player = Player { gold: 100, attributes: Vec::new() }
    let check = StatCheck { stat_name: "strength", min_value: 10 }
    let result = check.passes(player)
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "All categories should be fixed:\n{:?}",
        result.err()
    );
}
