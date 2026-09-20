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

//! P3.402 Product: `editor/asset_browser.wj` — `path.split('.')` emits `path.split(&'.')`
//! which fails rustc E0277 (Pattern expects char/&str, not &char).
//!
//! Tip gen: `if let Some(ext) = path.split(&'.').last() {`

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn file_ext(path: string) -> string {
    if let Some(ext) = path.split('.').last() {
        return ext
    }
    ""
}
"#;

#[test]
fn str_split_char_literal_must_not_emit_amp_char() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.402 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.402 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("split(&'.')") && !rs.contains("split(&\".\")"),
        "P3.402 RED: split char literal must not emit &char / &str-ref:\n{rs}"
    );
    assert!(
        rs.contains("split('.')") || rs.contains("split(\".\")"),
        "P3.402: expected bare char/str split delimiter:\n{rs}"
    );
    test.cargo_check().expect("P3.402 cargo-check");
}

#[test]
fn tip_out_game_core_asset_browser_split_must_not_emit_amp_char() {
    let roots = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("windjammer-game/windjammer-game-core/gen/editor/asset_browser.rs"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("windjammer-game/windjammer-game-core/editor/asset_browser.rs"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("windjammer-game/windjammer-game-core/gen/editor/scene_saver.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &roots {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("read");
        if text.contains("split(&'.')") || text.contains("split(&'/')") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "P3.402: game-core split sites missing");
    assert!(
        bad_paths.is_empty(),
        "P3.402 RED: tip gen still emits split(&char) in:\n  {}",
        bad_paths.join("\n  ")
    );
}
