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

//!
//!   `{ let __wj_tmp0 = InputEvent::key_down(…); self.recording.add_event(__wj_tmp0) }`
//!   `{ let __wj_tmp0 = self.cells[i].clone().get_id(); self.loaded_cells.push(__wj_tmp0) }`
//! Prefer direct `self.recording.add_event(InputEvent::key_down(…))`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Evt {
    pub code: i32,
}

pub struct Recording {
    pub events: Vec<Evt>,
}

pub struct Recorder {
    pub recording: Recording,
    pub current_frame: i32,
}

impl Evt {
    pub fn key_down(frame: i32, code: i32) -> Evt {
        Evt { code: code }
    }
}

impl Recording {
    pub fn add_event(self, evt: Evt) {
        self.events.push(evt)
    }
}

impl Recorder {
    pub fn record_key(self, code: i32) {
        self.recording.add_event(Evt::key_down(self.current_frame, code))
    }
}
"#;

#[test]
fn module_file_must_not_emit_wj_tmp_lets() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("__wj_tmp");
    assert!(
        !bad,
        "RED: emitted __wj_tmp let-binding:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

