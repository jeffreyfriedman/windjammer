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

//! WDB-362: indexed `&self` method must not clone the element first.
//!
//! Product tip game-core `lod/lod_config.rs`:
//!   `self.levels[i].clone().mesh_id()` with `mesh_id(&self) -> MeshId`
//! Prefer `self.levels[i].mesh_id()`. Twin of WDB-359 (field) / WDB-358 (self).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Level {
    pub id: i32,
}

impl Level {
    pub fn mesh_id(self) -> i32 {
        self.id
    }
}

pub struct Lod {
    pub levels: Vec<Level>,
}

pub fn first_id(lod: Lod) -> i32 {
    if lod.levels.len() > 0 {
        lod.levels[0].mesh_id()
    } else {
        0
    }
}
"#;

#[test]
fn wdb362_module_file_index_method_must_not_clone_element() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-362 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-362 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone()") || rs.contains("].clone().");
    assert!(
        !bad,
        "WDB-362 RED: indexed method cloned element:\n{rs}"
    );
    test.cargo_check().expect("WDB-362 cargo-check");
}

#[test]
fn wdb362_tip_out_game_core_lod_must_not_clone_before_mesh_id() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("lod_config.rs"),
        tip.join("lod/lod_config.rs"),
        tip.join("tile_rule.rs"),
        tip.join("autotile/tile_rule.rs"),
        tip.join("clip.rs"),
        tip.join("animation/clip.rs"),
        game.join("gen/lod/lod_config.rs"),
        game.join("gen/autotile/tile_rule.rs"),
        game.join("gen/animation/clip.rs"),
        game.join("gen/animation/skeleton.rs"),
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
            line.contains("].clone().")
                && (line.contains("mesh_id(")
                    || line.contains("id()")
                    || line.contains("sample(")
                    || line.contains(".id"))
                && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-362: game-core/tip product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-362 RED: tip/product ].clone().method in:\n  {}",
        bad_paths.join("\n  ")
    );
}
