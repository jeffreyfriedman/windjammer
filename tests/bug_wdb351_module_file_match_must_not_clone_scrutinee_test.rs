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

//! WDB-351: `match expr.clone()` must not clone when matching by value is unnecessary.
//!
//! Product tip game-core cluster:
//!   `physics/jolt/world.rs`: `match body.shape.clone() { … }` (twice)
//!   `assets/loader.rs`: `match asset.format.clone()`
//! Prefer `match &body.shape` / `match body.shape` (move) / `match &asset.format`.
//! Distinct from WDB-349 (matches! Option presence).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum Shape {
    Sphere(f32),
    Box,
}

pub struct Body {
    pub shape: Shape,
}

pub fn shape_kind(body: Body) -> i32 {
    match body.shape {
        Shape::Sphere(_) => 0,
        Shape::Box => 1,
    }
}
"#;

#[test]
fn wdb351_module_file_match_must_not_clone_scrutinee() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-351 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-351 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("match ") && rs.contains(".clone()");
    assert!(
        !bad,
        "WDB-351 RED: match scrutinee cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-351 cargo-check");
}

#[test]
fn wdb351_tip_out_game_core_must_not_match_clone_scrutinee() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("world.rs"),
        tip.join("physics/jolt/world.rs"),
        tip.join("jolt/world.rs"),
        tip.join("loader.rs"),
        tip.join("assets/loader.rs"),
        game.join("gen/physics/jolt/world.rs"),
        game.join("gen/assets/loader.rs"),
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
            let t = line.trim_start();
            !t.starts_with("//")
                && (t.contains("match ") && t.contains(".clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-351: game-core/tip product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-351 RED: tip/product match …clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
