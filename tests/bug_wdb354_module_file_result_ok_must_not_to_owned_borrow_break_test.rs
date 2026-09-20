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

//! WDB-354: Result Ok value must not emit `.map(|v| v.to_owned())` borrow-break.
//!
//! Product tip game-core `assets/loader.rs`:
//!   `self.load(name.clone(), &path, size).map(|__v| __v.to_owned())`
//! Prefer move/identity of already-owned Ok payload. Distinct from WDB-348 clone().to_string.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Asset {
    pub name: string,
}

pub fn load(name: string) -> Result<Asset, string> {
    Ok(Asset { name: name })
}

pub fn load_owned(name: string) -> Result<Asset, string> {
    load(name)
}
"#;

#[test]
fn wdb354_module_file_result_ok_must_not_to_owned_borrow_break() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-354 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-354 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".to_owned()") || rs.contains("to_owned()");
    assert!(
        !bad,
        "WDB-354 RED: Result Ok emitted to_owned borrow-break:\n{rs}"
    );
    test.cargo_check().expect("WDB-354 cargo-check");
}

#[test]
fn wdb354_tip_out_game_core_loader_must_not_to_owned_borrow_break() {
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
            line.contains(".to_owned()") && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-354: game-core/tip loader missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-354 RED: tip/product .to_owned() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
