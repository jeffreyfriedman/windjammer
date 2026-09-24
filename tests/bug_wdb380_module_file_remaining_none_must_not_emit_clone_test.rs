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

//! WDB-380: remaining product `None.clone()` sites WDB-367's tip path-list missed.
//!
//! WDB-367 isolate + listed tip files went GREEN (P3.427), but tip-out still emits
//! `None.clone()` in:
//!   `tilemap.rs`, `voxel/voxel_scene.rs`, `animation/blend_tree.rs`,
//!   `dialogue_system.rs`, `editor/animation_timeline.rs`, `audio/music_system.rs`
//! Prefer bare `None` (Copy). Twin of WDB-367.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct MusicSystem {
    pub current_track: Option<string>,
    pub next_track: Option<string>,
}

pub fn new_music() -> MusicSystem {
    MusicSystem {
        current_track: None,
        next_track: None,
    }
}

pub fn stop(m: MusicSystem) {
    m.current_track = None
}

pub struct BlendTree {
    pub root: Option<i32>,
    pub crossfade: Option<f32>,
}

pub fn new_blend() -> BlendTree {
    BlendTree {
        root: None,
        crossfade: None,
    }
}
"#;

#[test]
fn wdb380_module_file_remaining_none_must_not_emit_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-380 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-380 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("None.clone()"),
        "WDB-380 RED: None emitted with .clone():\n{rs}"
    );
    test.cargo_check().expect("WDB-380 cargo-check");
}

#[test]
fn wdb380_tip_out_game_core_remaining_must_not_none_clone() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("tilemap.rs"),
        tip.join("voxel/voxel_scene.rs"),
        tip.join("animation/blend_tree.rs"),
        tip.join("dialogue_system.rs"),
        tip.join("editor/animation_timeline.rs"),
        tip.join("audio/music_system.rs"),
        game.join("gen/tilemap.rs"),
        game.join("gen/voxel/voxel_scene.rs"),
        game.join("gen/animation/blend_tree.rs"),
        game.join("gen/dialogue_system.rs"),
        game.join("gen/editor/animation_timeline.rs"),
        game.join("gen/audio/music_system.rs"),
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
            !line.trim_start().starts_with("//") && line.contains("None.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-380: remaining None.clone() product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-380 RED: tip/product None.clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
