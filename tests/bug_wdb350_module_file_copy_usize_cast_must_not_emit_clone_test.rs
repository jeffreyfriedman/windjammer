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

//! WDB-350: Copy `usize` after cast must not emit `.clone()`.
//!
//! Product tip game-core `save/manager.rs`:
//!   `slots.push((key as usize).clone())`
//! Prefer bare `(key as usize)`. Twin of WDB-343 (Copy i32 clone).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn collect_slots(keys: Vec<i32>) -> Vec<usize> {
    let mut slots: Vec<usize> = Vec::new()
    for key in keys {
        slots.push(key as usize)
    }
    slots
}
"#;

#[test]
fn wdb350_module_file_copy_usize_cast_must_not_emit_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-350 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-350 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()");
    assert!(
        !bad,
        "WDB-350 RED: Copy usize cast emitted .clone():\n{rs}"
    );
    test.cargo_check().expect("WDB-350 cargo-check");
}

#[test]
fn wdb350_tip_out_game_core_save_must_not_clone_usize_cast() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("manager.rs"),
        tip.join("save/manager.rs"),
        game.join("gen/save/manager.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("save manager");
        let bad = text.lines().any(|line| {
            line.contains("as usize).clone()") && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-350: game-core/tip save manager missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-350 RED: tip/product (as usize).clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
