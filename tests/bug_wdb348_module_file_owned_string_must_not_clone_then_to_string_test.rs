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

//! WDB-348: owned `String` field init must not emit `path.clone().to_string()`.
//!
//! Product tip game-core `assets/loader.rs`:
//!   `LoadedAsset { …, path: path.clone().to_string(), … }` with `path: String`.
//! Prefer move (`path`) or a single clone (`path.clone()`), never clone+to_string.
//! Related to WDB-340 (double to_string) but distinct pattern.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct LoadedAsset {
    pub name: string,
    pub path: string,
}

pub fn load(name: string, path: string) -> LoadedAsset {
    LoadedAsset {
        name: name,
        path: path,
    }
}
"#;

#[test]
fn wdb348_module_file_owned_string_must_not_clone_then_to_string() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-348 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-348 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone().to_string()");
    assert!(
        !bad,
        "WDB-348 RED: owned String emitted clone().to_string():\n{rs}"
    );
    test.cargo_check().expect("WDB-348 cargo-check");
}

#[test]
fn wdb348_tip_out_game_core_loader_must_not_clone_then_to_string() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("loader.rs"),
        tip.join("assets/loader.rs"),
        game.join("gen/assets/loader.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("loader");
        let bad = text.lines().any(|line| {
            line.contains(".clone().to_string()") && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-348: game-core/tip loader missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-348 RED: tip/product clone().to_string() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
