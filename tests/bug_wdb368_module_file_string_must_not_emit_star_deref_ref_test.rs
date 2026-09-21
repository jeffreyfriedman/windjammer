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

//! WDB-368: borrowed `string` into `&str` formal must not emit `&*ident`.
//!
//! Product tip game-core `dialogue_tree.rs`:
//!   `world.get_flag(&*flag)` / `world.get_variable(&*key)`
//! Prefer `world.get_flag(flag)` / `world.get_variable(key)` (auto-deref / coerce).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct World {
    pub ok: bool,
}

pub fn get_flag(world: World, flag: string) -> bool {
    world.ok
}

pub fn check(world: World, flag: string) -> bool {
    get_flag(world, flag)
}
"#;

#[test]
fn wdb368_module_file_string_must_not_emit_star_deref_ref() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-368 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-368 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("&*"),
        "WDB-368 RED: emitted &* deref-ref:\n{rs}"
    );
    test.cargo_check().expect("WDB-368 cargo-check");
}

#[test]
fn wdb368_tip_out_game_core_dialogue_must_not_star_deref_ref() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("dialogue_tree.rs"),
        tip.join("dialogue/dialogue_tree.rs"),
        game.join("gen/dialogue_tree.rs"),
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
            line.contains("&*") && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-368: dialogue_tree product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-368 RED: tip/product &* in:\n  {}",
        bad_paths.join("\n  ")
    );
}
