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

//! WDB-341: `Option<String>` / owned String must not emit `String::from(...).to_string()`.
//!
//! Product tip game-core `ai/bt_validation.rs`:
//!   `return Some(String::from("bt: …").to_string())` → redundant coerce.
//! Prefer `Some("bt: …".to_string())` or `Some(String::from("bt: …"))`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn missing_root() -> Option<string> {
    Some("bt: root node record missing")
}

pub fn unknown_child() -> Option<string> {
    Some("bt: child references unknown id")
}
"#;

#[test]
fn wdb341_module_file_option_string_must_not_string_from_then_to_string() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-341 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-341 MultiFile lib.rs:\n{rs}");
    let chained = rs.contains(").to_string()");
    assert!(
        !(rs.contains("String::from(") && chained),
        "WDB-341 RED: Option<String> emitted String::from(...).to_string():\n{rs}"
    );
    test.cargo_check().expect("WDB-341 cargo-check");
}

#[test]
fn wdb341_tip_out_game_core_bt_validation_must_not_string_from_then_to_string() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("bt_validation.rs"),
        tip.join("ai/bt_validation.rs"),
        game.join("gen/ai/bt_validation.rs"),
        game.join("ai/bt_validation.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("bt_validation");
        let bad = text.lines().any(|line| {
            line.contains("String::from(") && line.contains(".to_string()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-341: game-core/tip bt_validation missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-341 RED: tip/product String::from(...).to_string() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
