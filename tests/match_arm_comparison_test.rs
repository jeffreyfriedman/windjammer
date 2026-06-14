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

use std::fs;
use std::process::Command;

use crate::test_utils::cargo_check_generated;

/// Keeps temp dir alive for the rest of the test (drop order: tuple first field last if we use `_root` binding).
fn setup_wj_build_and_build_dir(wj_code: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let test_root = tempfile::tempdir().expect("tempdir");
    let test_dir = test_root.path();
    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, wj_code).expect("write test.wj");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", "--no-cargo", wj_file.to_str().unwrap()])
        .current_dir(test_dir)
        .output()
        .expect("Failed to run wj build");

    assert!(
        output.status.success(),
        "wj build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let build_dir = test_dir.join("build");
    (test_root, build_dir)
}

#[test]
fn test_match_arm_binding_comparison_copy_type_cloned_in_if() {
    // Problem: match self.cost.clone() { Cost::Gold(amount) => if gold >= amount { ... } }
    // where amount: i32 (Copy type from cloned enum)
    // User writes: if gold >= amount inside match arm
    // Bug: Compiler generates: if gold >= *amount (E0614: i32 cannot be dereferenced)
    // Fix: Don't add * for match arm bindings of Copy types in if conditions
    let wj_code = r#"
enum Cost {
    None,
    Gold(i32),
}

struct Player {
    gold: i32,
}

struct Choice {
    cost: Cost,
}

impl Choice {
    // TDD: Simulate the exact scenario from dialog.wj
    // User writes: pub fn apply_cost(self, game_state: GameState)
    // Analyzer infers: pub fn apply_cost(self, mut game_state: GameState)
    // Result: match self.cost gets auto-cloned to match self.cost
    // Bug: amount binding treated as &i32 instead of i32
    pub fn apply_cost(self, mut player: Player) -> bool {
        match self.cost {
            Cost::None => true,
            Cost::Gold(amount) => {
                if player.gold >= amount {
                    player.gold = player.gold - amount
                    true
                } else {
                    false
                }
            },
        }
    }
}

pub fn main() {
    let mut player = Player { gold: 100 }
    let choice = Choice { cost: Cost::Gold(50) }
    let result = choice.apply_cost(player)
}
"#;

    let (_root, build_dir) = setup_wj_build_and_build_dir(wj_code);

    let rs_file = build_dir.join("test.rs");
    let generated_code = fs::read_to_string(&rs_file).unwrap();

    // Should NOT add * for Copy type match arm bindings (critical!)
    assert!(
        !generated_code.contains("*amount"),
        "Expected 'amount' without deref for Copy type, but found '*amount'. Generated code:\n{}",
        generated_code
    );

    // Should compare directly in if condition
    assert!(
        generated_code.contains("if player.gold >= amount"),
        "Expected 'if player.gold >= amount', got:\n{}",
        generated_code
    );

    // Should subtract directly (no deref) - either as compound assignment or regular subtraction
    assert!(
        generated_code.contains("player.gold - amount")
            || generated_code.contains("player.gold -= amount"),
        "Expected 'player.gold - amount' or 'player.gold -= amount', got:\n{}",
        generated_code
    );

    cargo_check_generated(&build_dir);
}

#[test]
fn test_match_arm_binding_comparison_copy_type() {
    // Control test: Direct match without .clone()
    let wj_code = r#"
enum Cost {
    None,
    Gold(i32),
}

struct Player {
    gold: i32,
}

pub fn can_afford(player: Player, cost: Cost) -> bool {
    match cost {
        Cost::None => true,
        Cost::Gold(amount) => {
            player.gold >= amount
        },
    }
}

pub fn main() {
    let player = Player { gold: 100 }
    let cost = Cost::Gold(50)
    let result = can_afford(player, cost)
}
"#;

    let (_root, build_dir) = setup_wj_build_and_build_dir(wj_code);

    let rs_file = build_dir.join("test.rs");
    let generated_code = fs::read_to_string(&rs_file).unwrap();

    // Should NOT add * for Copy type match arm bindings
    assert!(
        !generated_code.contains("*amount"),
        "Expected 'amount' without deref for Copy type, got:\n{}",
        generated_code
    );

    // Should compare directly
    assert!(
        generated_code.contains("player.gold >= amount"),
        "Expected 'player.gold >= amount', got:\n{}",
        generated_code
    );

    cargo_check_generated(&build_dir);
}

#[test]
fn test_match_arm_binding_comparison_string() {
    // Control test: String (non-Copy) should work correctly
    let wj_code = r#"
enum Cost {
    None,
    Item(string, i32),
}

struct Inventory {
    items: Vec<string>,
}

impl Inventory {
    pub fn has_item(self, item: string) -> bool {
        for stored_item in self.items {
            if stored_item == item {
                return true
            }
        }
        false
    }
}

pub fn can_afford(inventory: Inventory, cost: Cost) -> bool {
    match cost {
        Cost::None => true,
        Cost::Item(item_id, qty) => {
            inventory.has_item(item_id)
        },
    }
}

pub fn main() {
    let inventory = Inventory { items: Vec::new() }
    let cost = Cost::Item("sword".to_string(), 1)
    let result = can_afford(inventory, cost)
}
"#;

    let (_root, build_dir) = setup_wj_build_and_build_dir(wj_code);
    cargo_check_generated(&build_dir);
}
