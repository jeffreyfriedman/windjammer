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

//! WDB-355: Copy `Vec3` must not emit `.clone()` into owned formals.
//!
//! Product tip game-core `rendering/placeholder_assets.rs`:
//!   `vertex(…, n.clone(), …)` with `n: Vec3` (Copy) and `normal: Vec3`.
//! Prefer bare `n`. Twin of WDB-344/346 (f32 / field Copy).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub fn vertex(pos: Vec3, normal: Vec3) -> Vec3 {
    Vec3 {
        x: pos.x + normal.x,
        y: pos.y + normal.y,
        z: pos.z + normal.z,
    }
}

pub fn face(n: Vec3, p: Vec3) -> Vec3 {
    vertex(p, n)
}
"#;

#[test]
fn wdb355_module_file_copy_vec3_must_not_emit_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-355 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-355 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()");
    assert!(
        !bad,
        "WDB-355 RED: Copy Vec3 emitted .clone():\n{rs}"
    );
    test.cargo_check().expect("WDB-355 cargo-check");
}

#[test]
fn wdb355_tip_out_game_core_placeholder_must_not_clone_vec3() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("placeholder_assets.rs"),
        tip.join("rendering/placeholder_assets.rs"),
        game.join("gen/rendering/placeholder_assets.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("placeholder");
        let bad = text.lines().any(|line| {
            line.contains("n.clone()") && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-355: game-core/tip placeholder_assets missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-355 RED: tip/product clones Copy Vec3 in:\n  {}",
        bad_paths.join("\n  ")
    );
}
