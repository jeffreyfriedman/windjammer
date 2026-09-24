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

//! WDB-372: indexed enum field match must not clone scrutinee (`].value.clone()`).
//!
//! Product tip game-core `behavior_tree/blackboard.rs`:
//!   `match self.entries[idx as usize].value.clone() { BlackboardValue::Float(v) => … }`
//! Prefer `match &self.entries[idx].value` / `match self.entries[idx].value`.
//! Twin of WDB-351 (match clone scrutinee) + WDB-359 (index field).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum Val {
    Float(f32),
    Bool(bool),
    Text(string),
}

pub struct Entry {
    pub key: string,
    pub value: Val,
}

pub struct Board {
    pub entries: Vec<Entry>,
}

impl Board {
    pub fn get_f32(self, idx: usize) -> Option<f32> {
        match self.entries[idx].value {
            Val::Float(v) => Some(v),
            _ => None,
        }
    }
}
"#;

#[test]
fn wdb372_module_file_index_enum_match_must_not_clone_scrutinee() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-372 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-372 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".value.clone()");
    assert!(
        !bad,
        "WDB-372 RED: indexed enum match cloned scrutinee:\n{rs}"
    );
    test.cargo_check().expect("WDB-372 cargo-check");
}

#[test]
fn wdb372_tip_out_game_core_blackboard_must_not_clone_value_match() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("behavior_tree/blackboard.rs"),
        tip.join("blackboard.rs"),
        game.join("gen/behavior_tree/blackboard.rs"),
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
                && line.contains("match")
                && line.contains(".value.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-372: blackboard product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-372 RED: tip/product match .value.clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
