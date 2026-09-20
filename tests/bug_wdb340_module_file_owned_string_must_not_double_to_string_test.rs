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

//! WDB-340: owned String store must not emit `.to_string().to_string()`.
//!
//! Product tip game-core cluster:
//!   `ai/bt_serialization.rs`: `rec.params = value.to_string().to_string()`
//!   `scripting/components.rs`, `editor/prefab_system.rs`, `editor/asset_browser.rs`,
//!   `rendering/sprite.rs` — same double coerce.
//! Prefer a single `.to_string()` (from `&str`) or a move (from `String`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Rec {
    pub params: string,
}

pub fn update_params(value: string) -> Rec {
    Rec {
        params: value,
    }
}
"#;

#[test]
fn wdb340_module_file_owned_string_must_not_double_to_string() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-340 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-340 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".to_string().to_string()");
    assert!(
        !bad,
        "WDB-340 RED: owned String store emitted double to_string:\n{rs}"
    );
    test.cargo_check().expect("WDB-340 cargo-check");
}

#[test]
fn wdb340_tip_out_game_core_must_not_double_to_string() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("bt_serialization.rs"),
        tip.join("ai/bt_serialization.rs"),
        tip.join("components.rs"),
        tip.join("scripting/components.rs"),
        tip.join("prefab_system.rs"),
        tip.join("editor/prefab_system.rs"),
        tip.join("asset_browser.rs"),
        tip.join("editor/asset_browser.rs"),
        tip.join("sprite.rs"),
        tip.join("rendering/sprite.rs"),
        game.join("gen/ai/bt_serialization.rs"),
        game.join("gen/scripting/components.rs"),
        game.join("gen/editor/prefab_system.rs"),
        game.join("gen/editor/asset_browser.rs"),
        game.join("gen/rendering/sprite.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        if text.contains(".to_string().to_string()") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-340: game-core/tip product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-340 RED: tip/product double to_string in:\n  {}",
        bad_paths.join("\n  ")
    );
}
