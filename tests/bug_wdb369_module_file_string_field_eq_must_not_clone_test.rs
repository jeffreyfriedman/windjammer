#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
    feature = "codegen_tests",
))]

//! WDB-369: string field equality must not clone the field.
//!
//! Product tip game-core:
//!   `self.entries[i].key.clone() == key` (blackboard / localization)
//! Prefer `self.entries[i].key == key` (PartialEq for String/&str).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Entry {
    pub key: string,
    pub value: i32,
}

pub struct Board {
    pub entries: Vec<Entry>,
}

pub fn has_key(board: Board, key: string) -> bool {
    let mut i = 0
    while i < board.entries.len() {
        if board.entries[i].key == key {
            return true
        }
        i = i + 1
    }
    false
}
"#;

#[test]
fn wdb369_module_file_string_field_eq_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-369 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-369 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".key.clone()") && rs.contains("==");
    assert!(
        !bad,
        "WDB-369 RED: string field eq cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-369 cargo-check");
}

#[test]
fn wdb369_tip_out_game_core_must_not_clone_string_field_eq() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("behavior_tree/blackboard.rs"),
        tip.join("blackboard.rs"),
        tip.join("localization/localization_manager.rs"),
        tip.join("localization_manager.rs"),
        game.join("gen/behavior_tree/blackboard.rs"),
        game.join("gen/localization/localization_manager.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            !line.trim_start().starts_with("//")
                && line.contains(".clone()")
                && line.contains("==")
                && (line.contains(".key") || line.contains(".lang_code"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-369: game-core/tip product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-369 RED: tip/product string-field .clone() == in:\n  {}",
        bad_paths.join("\n  ")
    );
}
