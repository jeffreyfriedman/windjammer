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

//! WDB-401: Copy `StreamState::Loading` must not emit `.clone()` (WDB-384 path miss).
//!
//! WDB-384/392/397 cover FaceDirection / Direction / TileType; product
//! gen/vgs/streaming.rs and gen/audio/streaming.rs still have
//! `StreamState::Loading.clone()` / `StreamState::Playing.clone()`.
//! WJ source is `self.entries[idx].state = StreamState::Loading`.
//! Numbered 401 because 395–400 are reserved or used by the compiler agent.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum StreamState {
    NotLoaded,
    Loading,
    Resident,
    Evicting,
    Playing,
}

pub struct Entry {
    pub state: StreamState,
}

pub fn mark_loading() -> Entry {
    Entry {
        state: StreamState::Loading,
    }
}

pub fn mark_resident() -> Entry {
    Entry {
        state: StreamState::Resident,
    }
}

pub fn mark_playing() -> Entry {
    Entry {
        state: StreamState::Playing,
    }
}
"#;

#[test]
fn wdb401_module_file_streamstate_unit_variant_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-401 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-401 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("StreamState::Loading.clone()")
        || rs.contains("StreamState::Resident.clone()")
        || rs.contains("StreamState::NotLoaded.clone()")
        || rs.contains("StreamState::Evicting.clone()")
        || rs.contains("StreamState::Playing.clone()");
    assert!(
        !bad,
        "WDB-401 RED: Copy StreamState unit variant cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-401 cargo-check");
}

#[test]
fn wdb401_tip_out_game_core_streaming_must_not_clone_streamstate() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("vgs/streaming.rs"),
        tip.join("audio/streaming.rs"),
        tip.join("streaming.rs"),
        game.join("gen/vgs/streaming.rs"),
        game.join("gen/audio/streaming.rs"),
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
                && line.contains("StreamState::")
                && line.contains(".clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-401: streaming product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-401 RED: tip/product StreamState::*.clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
