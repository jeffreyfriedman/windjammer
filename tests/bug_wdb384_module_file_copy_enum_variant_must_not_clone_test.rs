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

//! WDB-384: Copy enum unit variants must not emit `.clone()`.
//!
//! Product tip game-core:
//!   `FaceDirection::PosX.clone()` (mesh_generator)
//!   `AlertLevel::Suspicious.clone()` / `NPCBehavior::Patrol.clone()`
//!   `ChunkLifecycleState::Loading.clone()` / `MaterialNodeKind::ColorNode.clone()`
//! WJ source is `FaceDirection::PosX` (Copy). Prefer the bare variant.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum Face {
    PosX,
    NegX,
}

pub fn east() -> Face {
    Face::PosX
}

pub fn assign(cur: Face) -> Face {
    Face::NegX
}
"#;

#[test]
fn wdb384_module_file_copy_enum_variant_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-384 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-384 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("Face::PosX.clone()") || rs.contains("Face::NegX.clone()");
    assert!(
        !bad,
        "WDB-384 RED: Copy enum variant cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-384 cargo-check");
}

#[test]
fn wdb384_tip_out_game_core_must_not_clone_copy_enum_variant() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("rendering/mesh_generator.rs"),
        tip.join("mesh_generator.rs"),
        tip.join("ai/squad_tactics.rs"),
        tip.join("ai/npc_behavior.rs"),
        tip.join("world/streaming.rs"),
        tip.join("editor/material_editor.rs"),
        game.join("gen/rendering/mesh_generator.rs"),
        game.join("gen/ai/squad_tactics.rs"),
        game.join("gen/ai/npc_behavior.rs"),
        game.join("gen/world/streaming.rs"),
        game.join("gen/editor/material_editor.rs"),
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
                && (line.contains("FaceDirection::") && line.contains(".clone()")
                    || line.contains("AlertLevel::") && line.contains(".clone()")
                    || line.contains("NPCBehavior::") && line.contains(".clone()")
                    || line.contains("ChunkLifecycleState::") && line.contains(".clone()")
                    || line.contains("MaterialNodeKind::") && line.contains(".clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-384: Copy-enum product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-384 RED: tip/product Copy enum variant .clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
