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

//! WDB-374: indexed Copy enum field must not emit `].clone().state.clone()`.
//!
//! Product tip game-core `world/streaming.rs`:
//!   `chunk_lifecycle_is_unloaded(self.chunks[i].clone().state.clone())`
//! WJ source is `self.chunks[i].state` (Copy `ChunkLifecycleState`). Twin of WDB-370.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum State {
    Unloaded,
    Loaded,
}

pub struct Chunk {
    pub state: State,
    pub name: string,
}

pub fn is_unloaded(s: State) -> bool {
    match s {
        State::Unloaded => true,
        State::Loaded => false,
    }
}

pub struct World {
    pub chunks: Vec<Chunk>,
}

impl World {
    pub fn first_unloaded(self, i: usize) -> bool {
        is_unloaded(self.chunks[i].state)
    }
}
"#;

#[test]
fn wdb374_module_file_copy_enum_field_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-374 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-374 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().state") || rs.contains(".state.clone()");
    assert!(
        !bad,
        "WDB-374 RED: indexed Copy enum field cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-374 cargo-check");
}

#[test]
fn wdb374_tip_out_game_core_streaming_must_not_double_clone_state() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("world/streaming.rs"),
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
                && (line.contains("].clone().state") || line.contains(".state.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-374: streaming product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-374 RED: tip/product indexed state clone in:\n  {}",
        bad_paths.join("\n  ")
    );
}
