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

/// TDD Test: Dialog ownership inference bug
///
/// When a method parameter is inferred as borrowed (&GameState), and match arm bindings
/// are owned (String), the auto-borrow logic should still add & to convert String → &str
/// when calling methods that expect &str.
///
/// Current bug: The & is NOT added, causing E0308 errors.
#[test]
fn test_dialog_borrowed_game_state_pattern() {
    use std::fs;
    use std::process::Command;

    // This replicates the exact pattern from dialog.wj where game_state is inferred as &GameState
    let test_code = r#"
enum DialogCondition {
    HasItem(string, i32),
    HasGold(i32),
}

struct PlayerState {
    pub gold: i32,
}

struct Inventory {
    pub items: Vec<(string, i32)>,
}

impl Inventory {
    pub fn has_item(self, item_id: string, min_quantity: i32) -> bool {
        for (id, qty) in self.items {
            if *id == item_id {
                return *qty >= min_quantity
            }
        }
        false
    }
}

struct GameState {
    pub player: PlayerState,
    pub inventory: Inventory,
}

impl DialogCondition {
    pub fn evaluate(self, game_state: GameState) -> bool {
        match self {
            DialogCondition::HasItem(item_id, qty) => {
                game_state.inventory.has_item(item_id, qty)
            },
            DialogCondition::HasGold(amount) => {
                game_state.player.gold >= amount
            },
        }
    }
}
"#;

    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let out_dir = temp_dir.path().join("out");
    fs::create_dir_all(&out_dir).unwrap();
    let test_file = temp_dir.path().join("dialog_test.wj");
    fs::write(&test_file, test_code).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            test_file.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj build");

    assert!(output.status.success(), "wj build should succeed");

    let generated_rs = out_dir.join("dialog_test.rs");
    assert!(generated_rs.exists(), "Generated Rust file should exist");

    let generated_code = fs::read_to_string(&generated_rs).unwrap();

    // Print for debugging
    eprintln!("=== GENERATED CODE ===");
    eprintln!("{}", generated_code);

    // Check if & was added to item_id (the String match arm binding → &str conversion)
    let has_borrow_on_item_id = generated_code.contains("has_item(&item_id,");
    let has_no_borrow = generated_code.contains("has_item(item_id,")
        && !generated_code.contains("has_item(&item_id,");

    if has_no_borrow {
        eprintln!("ERROR: String → &str conversion not applied!");
        eprintln!("Expected: has_item(&item_id, ...)");
        eprintln!("Found: has_item(item_id, ...)");
    }

    assert!(
        has_borrow_on_item_id,
        "Should auto-add & to convert String → &str even when game_state is borrowed"
    );
}

#[test]
fn test_dialog_multiple_string_params() {
    use std::fs;
    use std::process::Command;

    let test_code = r#"
enum DialogCondition {
    AttributeCheck(string, i32),
    QuestComplete(string),
}

struct PlayerState {
}

impl PlayerState {
    pub fn get_attribute(self, name: string) -> i32 {
        0
    }
}

struct QuestLog {
}

impl QuestLog {
    pub fn is_quest_complete(self, quest_id: string) -> bool {
        false
    }
}

struct GameState {
    pub player: PlayerState,
    pub quest_log: QuestLog,
}

impl DialogCondition {
    pub fn evaluate(self, game_state: GameState) -> bool {
        match self {
            DialogCondition::AttributeCheck(attr, min) => {
                game_state.player.get_attribute(attr) >= min
            },
            DialogCondition::QuestComplete(quest_id) => {
                game_state.quest_log.is_quest_complete(quest_id)
            },
        }
    }
}
"#;

    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let out_dir = temp_dir.path().join("out");
    fs::create_dir_all(&out_dir).unwrap();
    let test_file = temp_dir.path().join("multi_string_test.wj");
    fs::write(&test_file, test_code).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            test_file.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj build");

    assert!(output.status.success(), "wj build should succeed");

    let generated_rs = out_dir.join("multi_string_test.rs");
    let generated_code = fs::read_to_string(&generated_rs).unwrap();

    eprintln!("=== GENERATED CODE ===");
    eprintln!("{}", generated_code);

    // Check both string parameters get &
    assert!(
        generated_code.contains("game_state.player.get_attribute(&attr)"),
        "Should auto-add & to attr"
    );
    assert!(
        generated_code.contains("game_state.quest_log.is_quest_complete(&quest_id)"),
        "Should auto-add & to quest_id"
    );
}
