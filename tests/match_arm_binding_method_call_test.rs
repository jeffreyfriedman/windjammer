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

// TDD Test: Match arm bindings (owned String) passed to methods expecting &String
// Reproduces E0308 errors in dialog.wj lines 27, 33, 36, 39, 146, etc.
//
// PROBLEM:
// Match arm bindings extract OWNED values from enum variants.
// When passing to methods that expect &String, we need to auto-borrow (add &).
//
// ROOT CAUSE: Forward reference problem!
// - evaluate() is defined at line 27, calls has_item()
// - has_item() is defined at line 425 (LATER in file)
// - When generating evaluate(), has_item signature not registered yet
// - Signature lookup fails → no auto-borrow → E0308
//
// EXAMPLE:
// match self {
//     HasItem(item_id, qty) => inventory.has_item(item_id, qty)
//                                                 ^^^^^^^
//     // ERROR: expected `&String`, found `String`
// }
//
// SOLUTION:
// Two-pass compilation or fallback heuristic:
// 1. Pass 1: Register all method signatures
// 2. Pass 2: Generate code with full signature knowledge
// OR: Use type inference to determine &String is needed

use std::fs;
use std::process::Command;

use crate::test_utils::cargo_check_generated;

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
fn test_match_arm_binding_auto_borrow_for_method() {
    let wj_code = r#"
enum Condition {
    HasItem(string, i32),
}

struct GameState {
    inventory: Inventory,
}

// TDD: This function is defined BEFORE has_item method (forward reference!)
pub fn check_condition(condition: Condition, game_state: GameState) -> bool {
    match condition {
        Condition::HasItem(item_id, qty) => {
            game_state.inventory.has_item(item_id, qty)
        },
    }
}

// TDD: Inventory and has_item defined AFTER check_condition (like dialog.wj)
struct Inventory {
    items: Vec<(string, i32)>,
}

impl Inventory {
    pub fn has_item(self, item_id: string, qty: i32) -> bool {
        for (id, count) in self.items {
            if *id == item_id && *count >= qty {
                return true
            }
        }
        false
    }
}
"#;

    let (_root, build_dir) = setup_wj_build_and_build_dir(wj_code);

    let rs_file = build_dir.join("test.rs");
    let generated_code = fs::read_to_string(&rs_file).unwrap();

    // Match arm bindings (owned String) should be auto-borrowed when passed to &String params
    // Field chain calls (game_state.inventory.has_item) should also work
    assert!(
        generated_code.contains("game_state.inventory.has_item(&item_id, qty)")
            || generated_code.contains("game_state.inventory.has_item(&*item_id, qty)"),
        "Expected auto-borrow for match arm binding 'item_id' in field chain call, got:\n{}",
        generated_code
    );

    cargo_check_generated(&build_dir);
}

#[test]
fn test_match_arm_binding_multiple_string_params() {
    let wj_code = r#"
enum Event {
    Interaction(string, string), // npc_id, dialog_id
}

struct DialogManager {
    dialogs: Vec<string>,
}

impl DialogManager {
    pub fn start_dialog(self, npc_id: string, dialog_id: string) -> bool {
        true
    }
}

pub fn handle_event(event: Event, manager: DialogManager) -> bool {
    match event {
        Event::Interaction(npc_id, dialog_id) => {
            manager.start_dialog(npc_id, dialog_id)
        },
    }
}
"#;

    let (_root, build_dir) = setup_wj_build_and_build_dir(wj_code);

    let rs_file = build_dir.join("test.rs");
    let generated_code = fs::read_to_string(&rs_file).unwrap();

    // Both match arm bindings should be auto-borrowed
    assert!(
        (generated_code.contains("manager.start_dialog(&npc_id, &dialog_id)")
            || generated_code.contains("manager.start_dialog(&*npc_id, &*dialog_id)")),
        "Expected auto-borrow for both match arm bindings, got:\n{}",
        generated_code
    );

    cargo_check_generated(&build_dir);
}
