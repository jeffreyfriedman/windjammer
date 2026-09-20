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

//! WDB-332: i32 formal must not receive `priority.to_string()`.
//!
//! Product tip game-core `audio/audio_mixer.rs`:
//!   `AudioChannel::new(id, priority.to_string())` with `priority: i32` → E0308.
//! Root cause: sibling `audio/mixer.wj` defines a different `AudioChannel::new(id, name: string)`
//! that wins leaf-name resolution via stale-owned heuristics. Prefer caller-module affinity.
//! Source WJ: `AudioChannel::new(id, priority)`. Distinct from P3.401 `(name as i32)`
//! sibling-poison on string-name `new`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
mod mixer
mod audio_mixer
"#;

const MIXER: &str = r#"
pub struct AudioChannel {
    pub id: i32,
    pub name: string,
}

impl AudioChannel {
    pub fn new(id: i32, name: string) -> AudioChannel {
        AudioChannel { id: id, name: name }
    }
}
"#;

const AUDIO_MIXER: &str = r#"
pub type ChannelId = i32

pub struct AudioChannel {
    pub id: ChannelId,
    pub priority: i32,
}

impl AudioChannel {
    pub fn new(id: ChannelId, priority: i32) -> AudioChannel {
        AudioChannel { id: id, priority: priority }
    }
}

pub struct AudioMixer {
    pub channels: Vec<AudioChannel>,
}

impl AudioMixer {
    pub fn add_channel(self, priority: i32) -> ChannelId {
        let id = self.channels.len() as i32
        self.channels.push(AudioChannel::new(id, priority))
        id
    }
}
"#;

#[test]
fn wdb332_module_file_i32_formal_must_not_receive_to_string() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    test.add_file("mixer.wj", MIXER);
    test.add_file("audio_mixer.wj", AUDIO_MIXER);
    let map = test.compile().expect("WDB-332 compile");
    let rs = map
        .get("audio_mixer.rs")
        .or_else(|| map.get("lib.rs"))
        .expect("audio_mixer.rs or lib.rs");
    eprintln!("WDB-332 MultiFile audio_mixer.rs:\n{rs}");
    let call_bad = rs.contains("priority.to_string()") || rs.contains(", priority.to_string()");
    assert!(
        !call_bad,
        "WDB-332 RED: i32 formal received priority.to_string():\n{rs}"
    );
    test.cargo_check().expect("WDB-332 cargo-check");
}

#[test]
fn wdb332_tip_out_game_core_audio_mixer_must_not_to_string_i32_priority() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("audio_mixer.rs"),
        tip.join("audio/audio_mixer.rs"),
        game.join("gen/audio/audio_mixer.rs"),
        game.join("audio/audio_mixer.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("audio_mixer");
        let bad = text.lines().any(|line| {
            line.contains("AudioChannel::new(") && line.contains("to_string()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-332: game-core/tip audio_mixer missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-332 RED: tip/product passes priority.to_string() into i32 in:\n  {}",
        bad_paths.join("\n  ")
    );
}
