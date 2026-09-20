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

//! WDB-344: Copy `f32` must not emit `.clone()` on locals.
//!
//! Product tip game-core `physics/collision2d.rs`:
//!   `Vec2::new(normal_x.clone(), 0.0)` / `normal_y.clone()` with `normal_*: f32`.
//! Prefer bare Copy values.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

pub fn mk_normal(normal_x: f32, normal_y: f32) -> Vec2 {
    Vec2 {
        x: normal_x,
        y: normal_y,
    }
}
"#;

#[test]
fn wdb344_module_file_copy_f32_must_not_emit_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-344 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-344 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()");
    assert!(
        !bad,
        "WDB-344 RED: Copy f32 emitted .clone():\n{rs}"
    );
    test.cargo_check().expect("WDB-344 cargo-check");
}

#[test]
fn wdb344_tip_out_game_core_collision2d_must_not_clone_f32() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("collision2d.rs"),
        tip.join("physics/collision2d.rs"),
        game.join("gen/physics/collision2d.rs"),
        game.join("physics/collision2d.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("collision2d");
        let bad = text.lines().any(|line| {
            (line.contains("normal_x.clone()") || line.contains("normal_y.clone()"))
                && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-344: game-core/tip collision2d missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-344 RED: tip/product clones Copy f32 in:\n  {}",
        bad_paths.join("\n  ")
    );
}
