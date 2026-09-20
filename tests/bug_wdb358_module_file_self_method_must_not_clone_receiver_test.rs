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

//! WDB-358: `&self` method calls must not emit `self.clone().method()`.
//!
//! Product tip game-core cluster (non-Copy receivers):
//!   `streaming_coordinator.rs`: `self.clone().is_tile_active(id)` with `is_tile_active(&self)`
//!   `squad_tactics.rs`: `self.clone().get_alive_count()`
//!   `shader_graph_compiler.rs` / `mesh_renderer.rs` / `voxel_scene.rs` — same pattern
//! Prefer `self.method(…)`. Distinct from WDB-356 (Copy Mat4).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Streamer {
    pub active: Vec<u32>,
}

impl Streamer {
    pub fn is_tile_active(self, tile_id: u32) -> bool {
        let mut i: usize = 0
        while i < self.active.len() {
            if self.active[i] == tile_id {
                return true
            }
            i = i + 1
        }
        false
    }

    pub fn needs_center(self, center_id: u32) -> bool {
        !self.is_tile_active(center_id)
    }
}
"#;

#[test]
fn wdb358_module_file_self_method_must_not_clone_receiver() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-358 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-358 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("self.clone()");
    assert!(
        !bad,
        "WDB-358 RED: self.clone() before &self method:\n{rs}"
    );
    test.cargo_check().expect("WDB-358 cargo-check");
}

#[test]
fn wdb358_tip_out_game_core_must_not_self_clone_before_borrowed_method() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("streaming_coordinator.rs"),
        tip.join("world/streaming_coordinator.rs"),
        tip.join("squad_tactics.rs"),
        tip.join("ai/squad_tactics.rs"),
        tip.join("mesh_renderer.rs"),
        tip.join("rendering/mesh_renderer.rs"),
        game.join("gen/world/streaming_coordinator.rs"),
        game.join("gen/ai/squad_tactics.rs"),
        game.join("gen/rendering/mesh_renderer.rs"),
        game.join("gen/rendering/shader_graph_compiler.rs"),
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
            line.contains("self.clone().") && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-358: game-core/tip product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-358 RED: tip/product self.clone().method in:\n  {}",
        bad_paths.join("\n  ")
    );
}
