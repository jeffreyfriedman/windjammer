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

//! WDB-403: Copy `ChunkLifecycleState::Loading` must not emit `.clone()`.
//!
//! WDB-384/392/397/401 cover FaceDirection / Direction / TileType / StreamState;
//! product gen/world/streaming.rs still has `ChunkLifecycleState::Loading.clone()`.
//! WJ source is `self.chunks[i].state = ChunkLifecycleState::Loading`.
//! Numbered 403 to stay past compiler 395–400 and DB 401/402.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum ChunkLifecycleState {
    Unloaded,
    Loading,
    Loaded,
}

pub struct Chunk {
    pub state: ChunkLifecycleState,
}

pub fn mark_loading() -> Chunk {
    Chunk {
        state: ChunkLifecycleState::Loading,
    }
}

pub fn mark_unloaded() -> Chunk {
    Chunk {
        state: ChunkLifecycleState::Unloaded,
    }
}
"#;

#[test]
fn wdb403_module_file_chunklifecycle_unit_variant_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-403 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-403 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("ChunkLifecycleState::Loading.clone()")
        || rs.contains("ChunkLifecycleState::Unloaded.clone()")
        || rs.contains("ChunkLifecycleState::Loaded.clone()");
    assert!(
        !bad,
        "WDB-403 RED: Copy ChunkLifecycleState unit variant cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-403 cargo-check");
}

#[test]
fn wdb403_tip_out_game_core_world_streaming_must_not_clone_chunklifecycle() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("world/streaming.rs"),
        tip.join("streaming.rs"),
        game.join("gen/world/streaming.rs"),
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
                && line.contains("ChunkLifecycleState::")
                && line.contains(".clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-403: world streaming product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-403 RED: tip/product ChunkLifecycleState::*.clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
