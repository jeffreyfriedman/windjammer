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

//! WDB-363: indexed tuple/Copy field access must not clone the whole element.
//!
//! Product tip game-core `animation/blend_tree.rs`:
//!   `a[idx].clone().rotation.0` / `b[idx].clone().position` with Copy Vec3 / tuple fields
//! Prefer `a[idx].rotation.0`. Twin of WDB-359 (struct field).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Xform {
    pub rotation: (f32, f32, f32, f32),
}

pub fn rot0(xs: Vec<Xform>, idx: usize) -> f32 {
    xs[idx].rotation.0
}
"#;

#[test]
fn wdb363_module_file_index_tuple_field_must_not_clone_element() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-363 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-363 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone()");
    assert!(
        !bad,
        "WDB-363 RED: indexed tuple field cloned element:\n{rs}"
    );
    test.cargo_check().expect("WDB-363 cargo-check");
}

#[test]
fn wdb363_tip_out_game_core_blend_tree_must_not_clone_before_rotation() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("blend_tree.rs"),
        tip.join("animation/blend_tree.rs"),
        tip.join("uv_test.rs"),
        tip.join("editor/uv_test.rs"),
        tip.join("frustum_test.rs"),
        tip.join("frustum/frustum_test.rs"),
        game.join("gen/animation/blend_tree.rs"),
        game.join("gen/editor/uv_test.rs"),
        game.join("gen/frustum/frustum_test.rs"),
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
            (line.contains("].clone().rotation")
                || line.contains("].clone().position")
                || line.contains("].clone().u")
                || line.contains("].clone().v")
                || line.contains("].clone().normal"))
                && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-363: game-core/tip product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-363 RED: tip/product ].clone().field in:\n  {}",
        bad_paths.join("\n  ")
    );
}
