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

//! WDB-347: Copy `f32` match bindings must not emit `.clone()`.
//!
//! Product tip game-core `physics/jolt/world.rs`:
//!   `ShapeType::Sphere(r) => (r.clone(), 0.0, 0.0)`
//!   `Capsule(r, h) => (r.clone(), h.clone(), 0.0)`
//! Prefer bare Copy bindings. Distinct from WDB-344/346 (locals/fields).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum Shape {
    Sphere(f32),
    Capsule(f32, f32),
}

pub fn dims(shape: Shape) -> (f32, f32, f32) {
    match shape {
        Shape::Sphere(r) => (r, 0.0, 0.0),
        Shape::Capsule(r, h) => (r, h, 0.0),
    }
}
"#;

#[test]
fn wdb347_module_file_copy_f32_match_binding_must_not_emit_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-347 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-347 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()");
    assert!(
        !bad,
        "WDB-347 RED: Copy f32 match binding emitted .clone():\n{rs}"
    );
    test.cargo_check().expect("WDB-347 cargo-check");
}

#[test]
fn wdb347_tip_out_game_core_jolt_must_not_clone_f32_match_bindings() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("world.rs"),
        tip.join("physics/jolt/world.rs"),
        tip.join("jolt/world.rs"),
        game.join("gen/physics/jolt/world.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("jolt world");
        let bad = text.lines().any(|line| {
            (line.contains("r.clone()") || line.contains("h.clone()"))
                && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-347: game-core/tip jolt world missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-347 RED: tip/product clones Copy f32 match bindings in:\n  {}",
        bad_paths.join("\n  ")
    );
}
