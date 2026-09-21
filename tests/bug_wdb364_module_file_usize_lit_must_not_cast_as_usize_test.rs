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

//! WDB-364: `usize` literals must not emit redundant `N_usize as usize`.
//!
//! Product tip game-core cluster:
//!   `0_usize as usize`, `1_usize as usize`, `20_usize as usize`
//! Prefer bare `0_usize` / `1`. Twin of WDB-361 (`i as usize`) / WDB-352 (`_i32 as i32`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn clamp_idx(i: usize, parent: usize) -> bool {
    if parent >= 0 {
        true
    } else {
        i + 1 < parent
    }
}
"#;

#[test]
fn wdb364_module_file_usize_lit_must_not_cast_as_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-364 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-364 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("_usize as usize") || rs.contains("0_usize as usize");
    assert!(
        !bad,
        "WDB-364 RED: usize lit emitted as usize:\n{rs}"
    );
    test.cargo_check().expect("WDB-364 cargo-check");
}

#[test]
fn wdb364_tip_out_game_core_must_not_cast_usize_lit() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("skeleton.rs"),
        tip.join("animation/skeleton.rs"),
        tip.join("svo_debug.rs"),
        tip.join("voxel/svo_debug.rs"),
        tip.join("scene.rs"),
        tip.join("ecs/scene.rs"),
        tip.join("physics_world.rs"),
        tip.join("physics/physics_world.rs"),
        tip.join("spatial_index.rs"),
        tip.join("rendering/spatial_index.rs"),
        game.join("gen/animation/skeleton.rs"),
        game.join("gen/voxel/svo_debug.rs"),
        game.join("gen/ecs/scene.rs"),
        game.join("gen/physics/physics_world.rs"),
        game.join("gen/rendering/spatial_index.rs"),
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
            line.contains("_usize as usize") && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-364: game-core/tip product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-364 RED: tip/product _usize as usize in:\n  {}",
        bad_paths.join("\n  ")
    );
}
