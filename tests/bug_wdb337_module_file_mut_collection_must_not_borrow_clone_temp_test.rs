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

//! WDB-337: `&mut Vec`/`&mut Grid` formal must not receive `&mut quads.clone()` (temp).
//!
//! Product tip game-core cluster (twin of WDB-336 mesh_renderer):
//!   `voxel/meshing.rs`: `greedy_mesh_axis(&grid, &mut quads.clone(), …)`
//!   `scene/component_viewer_controls.rs`: `build_pedestal(&mut grid.clone())`
//!   `assets/vox_loader_test.rs`: `push_chunk_id(&mut d.clone(), …)` (42 hits)
//! → E0716 / discarded mutation.
//! Prefer `&mut quads` / `&mut grid` / `&mut d`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn greedy_mesh_axis_on(quads: Vec<i32>, axis: i32) {
    quads.push(axis)
}

// Product shape (meshing): local `let mut quads` reused into demoted `&mut Vec`
// must emit `&mut quads`, never `&mut quads.clone()`.
pub fn mesh_all() -> Vec<i32> {
    let mut quads: Vec<i32> = Vec::new()
    greedy_mesh_axis_on(quads, 0)
    greedy_mesh_axis_on(quads, 1)
    quads
}
"#;

#[test]
fn wdb337_module_file_mut_collection_must_not_borrow_clone_temp() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-337 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-337 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("&mut quads.clone()")
        || rs.contains("&mut grid.clone()")
        || (rs.contains("&mut ") && rs.contains(".clone()"));
    assert!(
        !bad,
        "WDB-337 RED: mut collection received &mut <temp>.clone():\n{rs}"
    );
    test.cargo_check().expect("WDB-337 cargo-check");
}

#[test]
fn wdb337_tip_out_game_core_meshing_viewer_vox_must_not_mut_borrow_clone_temp() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("meshing.rs"),
        tip.join("voxel/meshing.rs"),
        tip.join("component_viewer_controls.rs"),
        tip.join("scene/component_viewer_controls.rs"),
        tip.join("vox_loader_test.rs"),
        tip.join("assets/vox_loader_test.rs"),
        game.join("gen/voxel/meshing.rs"),
        game.join("gen/scene/component_viewer_controls.rs"),
        game.join("gen/assets/vox_loader_test.rs"),
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
            line.contains("&mut ") && line.contains(".clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-337: game-core/tip meshing/viewer/vox missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-337 RED: tip/product uses &mut <temp>.clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
