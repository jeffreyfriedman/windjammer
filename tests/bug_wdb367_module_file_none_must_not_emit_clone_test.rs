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

//! WDB-367: `None` must not emit `None.clone()`.
//!
//! Product tip game-core cluster (28+ sites):
//!   `src_x: None.clone()`, `current_state: None.clone()`, `best_idx: Option<usize> = None.clone()`
//! Prefer bare `None` (Copy). Twin of WDB-343 (Copy i32 `.clone()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Sprite {
    pub src_x: Option<f32>,
    pub src_y: Option<f32>,
}

pub struct StateId {
    pub id: i32,
}

pub struct Machine {
    pub current_state: Option<StateId>,
    pub previous_state: Option<StateId>,
    pub initial_state: Option<StateId>,
}

pub fn new_sprite() -> Sprite {
    Sprite {
        src_x: None,
        src_y: None,
    }
}

pub fn clear(s: Sprite) {
    s.src_x = None
}

pub fn clear_machine(m: Machine) {
    m.current_state = None
    m.previous_state = None
    m.initial_state = None
}
"#;

#[test]
fn wdb367_module_file_none_must_not_emit_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-367 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-367 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("None.clone()"),
        "WDB-367 RED: None emitted with .clone():\n{rs}"
    );
    test.cargo_check().expect("WDB-367 cargo-check");
}

#[test]
fn wdb367_tip_out_game_core_must_not_none_clone() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("sprite/sprite.rs"),
        tip.join("sprite.rs"),
        tip.join("state_machine/machine.rs"),
        tip.join("tilemap/tilemap.rs"),
        tip.join("ai/npc_behavior.rs"),
        tip.join("ai/perception.rs"),
        tip.join("ai/squad_tactics.rs"),
        tip.join("scene/manager.rs"),
        tip.join("rendering/texture_packer.rs"),
        tip.join("rendering/bvh.rs"),
        tip.join("rendering/mesh_generator.rs"),
        tip.join("dcc_pipeline/usd.rs"),
        game.join("gen/sprite/sprite.rs"),
        game.join("gen/state_machine/machine.rs"),
        game.join("gen/tilemap/tilemap.rs"),
        game.join("gen/ai/npc_behavior.rs"),
        game.join("gen/scene/manager.rs"),
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
            line.contains("None.clone()") && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-367: game-core/tip product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-367 RED: tip/product None.clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
