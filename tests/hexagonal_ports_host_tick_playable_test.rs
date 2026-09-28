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

//! Ports Host.tick_playable — fire spends ammo, observes kills, windjammer-ui HUD.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const STUB: &str = r#"
pub struct PlaytestCmd { pub fire_weapon: bool }
impl PlaytestCmd {
    pub fn fire() -> PlaytestCmd { PlaytestCmd { fire_weapon: true } }
}
pub struct FakeWorld { pub ammo: i32, pub kill_count: i32 }
impl FakeWorld {
    pub fn new() -> FakeWorld { FakeWorld { ammo: 30, kill_count: 0 } }
}
pub struct Host { pub world: FakeWorld, pub hud: string }
impl Host {
    pub fn new() -> Host { Host { world: FakeWorld::new(), hud: "" } }
    pub fn tick_playable(self, _cmd: PlaytestCmd, _dt: f32) {}
}
"#;

fn isolate(src: &str) -> String {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", src);
    let map = test.compile().expect("ports tick compile");
    let rs = map.get("lib.rs").expect("lib.rs").clone();
    test.cargo_check().expect("ports tick cargo-check");
    rs
}

#[test]
fn ports_host_tick_playable_stub_stays_red() {
    let rs = isolate(STUB);
    assert!(!rs.contains("ammo -= 1") && !rs.contains("ammo - 1"));
    assert!(!rs.contains("wj-panel"));
}

#[test]
fn ports_host_tick_playable_must_fire_observe_and_hud() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core/src/application/ecs_playable_loop.wj");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|_| STUB.to_string());
    let rs = isolate(&src);
    assert!(
        rs.contains("ammo -= 1") || rs.contains("ammo - 1"),
        "must spend ammo:\n{rs}"
    );
    assert!(
        rs.contains("kills += 1") || rs.contains("kills + 1"),
        "must increment kills:\n{rs}"
    );
    assert!(
        rs.contains("wj-panel") && rs.contains("wj-progress") && rs.contains("KILLS"),
        "must compose ui hud:\n{rs}"
    );
}
