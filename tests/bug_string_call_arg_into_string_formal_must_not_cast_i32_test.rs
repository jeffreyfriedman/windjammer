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

//! `audio/mixer.wj` + `audio/audio_mixer.wj` both define `AudioChannel::new`.
//! Same-file isolate is GREEN; multipass sibling with `new(id, priority: i32)` poisons
//! string-name call sites → `(name as i32)` (E0277).
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod mixer
pub mod audio_mixer
"#;

const MIXER: &str = r#"
pub struct AudioChannel {
    pub id: i32,
    pub name: string,
}

impl AudioChannel {
    pub fn new(id: i32, name: string) -> AudioChannel {
        AudioChannel {
            id: id,
            name: name,
        }
    }
}

pub struct AudioMixer {
    pub channels: Vec<AudioChannel>,
}

impl AudioMixer {
    pub fn add_channel(name: string) -> i32 {
        let id = self.channels.len() as i32
        self.channels.push(AudioChannel::new(id, name))
        id
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
        AudioChannel {
            id: id,
            priority: priority,
        }
    }
}
"#;

#[test]
fn string_call_arg_into_string_formal_must_not_cast_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("mixer.wj", MIXER);
    test.add_file("audio_mixer.wj", AUDIO_MIXER);
    let map = test.compile().expect("compile");
    let rs = map.get("mixer.rs").expect("mixer.rs");
    eprintln!("MultiFile mixer.rs:\n{rs}");
    assert!(
        !rs.contains("name as i32") && !rs.contains("(name as i32)"),
        "RED: sibling AudioChannel::new(i32) must not poison string-name call:\n{rs}"
    );
    assert!(
        rs.contains("AudioChannel::new(id, name)")
            || rs.contains("AudioChannel::new(id, name.clone())")
            || rs.contains("AudioChannel::new(id, name.to_string())"),
        "expected bare/owned name into mixer AudioChannel::new:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
