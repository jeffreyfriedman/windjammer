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

//! WDB-365: statement args must not emit `__wj_tmpN` let-bindings for simple passes.
//!
//! Product tip game-core cluster:
//!   `{ let __wj_tmp0 = InputEvent::key_down(…); self.recording.add_event(__wj_tmp0) }`
//!   `{ let __wj_tmp0 = self.cells[i].clone().get_id(); self.loaded_cells.push(__wj_tmp0) }`
//! Prefer direct `self.recording.add_event(InputEvent::key_down(…))`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Evt {
    pub code: i32,
}

pub struct Rec {
    pub events: Vec<Evt>,
}

pub fn key_down(code: i32) -> Evt {
    Evt { code: code }
}

pub fn record(rec: Rec, code: i32) {
    rec.events.push(key_down(code))
}
"#;

#[test]
fn wdb365_module_file_must_not_emit_wj_tmp_lets() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-365 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-365 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("__wj_tmp");
    assert!(
        !bad,
        "WDB-365 RED: emitted __wj_tmp let-binding:\n{rs}"
    );
    test.cargo_check().expect("WDB-365 cargo-check");
}

#[test]
fn wdb365_tip_out_game_core_must_not_emit_wj_tmp() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("input_recorder.rs"),
        tip.join("testing/input_recorder.rs"),
        tip.join("streaming.rs"),
        tip.join("world_partition/streaming.rs"),
        tip.join("dispatcher.rs"),
        tip.join("event/dispatcher.rs"),
        tip.join("dialogue_system.rs"),
        tip.join("manager.rs"),
        tip.join("scene/manager.rs"),
        game.join("gen/testing/input_recorder.rs"),
        game.join("gen/world_partition/streaming.rs"),
        game.join("gen/event/dispatcher.rs"),
        game.join("gen/dialogue_system.rs"),
        game.join("gen/scene/manager.rs"),
        game.join("gen/breach_protocol_game.rs"),
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
            line.contains("__wj_tmp") && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-365: game-core/tip product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-365 RED: tip/product __wj_tmp in:\n  {}",
        bad_paths.join("\n  ")
    );
}
