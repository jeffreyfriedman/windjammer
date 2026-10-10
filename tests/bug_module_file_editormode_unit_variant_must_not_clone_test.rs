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

//! Regression: module file editormode unit variant must not clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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
fn module_file_editormode_unit_variant_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("EditorMode::Pause.clone()")
        || rs.contains("EditorMode::Play.clone()")
        || rs.contains("EditorMode::Edit.clone()");
    assert!(
        !bad,
        "RED: Copy EditorMode unit variant cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
