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

//! WDB-404: Copy `EditorMode::Pause` must not emit `.clone()`.
//!
//! WDB-384/392/397/401/403 cover other Copy unit enums; product
//! gen/editor/editor_core.rs still has `EditorMode::Pause.clone()`.
//! WJ source is `self.mode = EditorMode::Pause`.
//! Numbered 404 to stay past compiler 395–400 and DB 401–403.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum EditorMode {
    Edit,
    Play,
    Pause,
}

pub struct EditorState {
    pub mode: EditorMode,
}

pub fn pause() -> EditorState {
    EditorState {
        mode: EditorMode::Pause,
    }
}

pub fn play() -> EditorState {
    EditorState {
        mode: EditorMode::Play,
    }
}
"#;

#[test]
fn wdb404_module_file_editormode_unit_variant_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-404 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-404 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("EditorMode::Pause.clone()")
        || rs.contains("EditorMode::Play.clone()")
        || rs.contains("EditorMode::Edit.clone()");
    assert!(
        !bad,
        "WDB-404 RED: Copy EditorMode unit variant cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-404 cargo-check");
}

#[test]
fn wdb404_tip_out_game_core_editor_core_must_not_clone_editormode() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("editor/editor_core.rs"),
        tip.join("editor_core.rs"),
        game.join("gen/editor/editor_core.rs"),
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
                && line.contains("EditorMode::")
                && line.contains(".clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-404: editor_core product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-404 RED: tip/product EditorMode::*.clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
