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

//! WDB-390: remaining ].clone().mesh_id() at gen/lod_config.rs (WDB-362 path miss).
//!
//! WDB-362 tip list checks gen/lod/lod_config.rs but product lives at
//! gen/lod_config.rs: return Some(self.levels[i].clone().mesh_id()).
//! WJ source is self.levels[i].mesh_id(). Twin of WDB-362/387.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct MeshId {
    pub id: i32,
}

pub struct Level {
    pub id: MeshId,
}

impl Level {
    pub fn mesh_id(self) -> MeshId {
        self.id
    }

    pub fn is_active(self, _d: f32) -> bool {
        true
    }
}

pub struct Lod {
    pub levels: Vec<Level>,
}

impl Lod {
    pub fn get_level_for_distance(self, distance: f32) -> Option<MeshId> {
        let mut i = 0
        while i < self.levels.len() {
            if self.levels[i].is_active(distance) {
                return Some(self.levels[i].mesh_id())
            }
            i = i + 1
        }
        None
    }
}
"#;

#[test]
fn wdb390_module_file_lod_config_mesh_id_must_not_clone_element() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-390 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-390 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("].clone().mesh_id()"),
        "WDB-390 RED: indexed mesh_id cloned the element:\n{rs}"
    );
    test.cargo_check().expect("WDB-390 cargo-check");
}

#[test]
fn wdb390_tip_out_game_core_lod_config_must_not_clone_before_mesh_id() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("lod_config.rs"),
        game.join("gen/lod_config.rs"),
        game.join("gen/lod/lod_config.rs"),
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
            !line.trim_start().starts_with("//") && line.contains("].clone().mesh_id()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-390: lod_config product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-390 RED: tip/product ].clone().mesh_id() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
