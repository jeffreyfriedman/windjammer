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

//! remaining product `None.clone()` sites 's tip path-list missed.
//!
//! isolate + listed tip files went GREEN (P3.427), but tip-out still emits
//! `None.clone()` in:
//!   `tilemap.rs`, `voxel/voxel_scene.rs`, `animation/blend_tree.rs`,
//!   `dialogue_system.rs`, `editor/animation_timeline.rs`, `audio/music_system.rs`
//! Prefer bare `None` (Copy). Twin of .

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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
fn module_file_remaining_none_must_not_emit_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("None.clone()"),
        "None emitted with .clone():\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
