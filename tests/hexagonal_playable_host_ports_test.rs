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

//! Hexagonal playable host — spec 2026-09-27.
//! Fake adapters + HUD composition without FFI.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

/// Stub host: time does not advance, compose emits no ui classes. TDD RED until implemented.
const STUB: &str = r#"
pub struct GameHudState {
    pub health: f32,
    pub max_health: f32,
    pub ammo: i32,
    pub objective: string,
}

impl GameHudState {
    pub fn new() -> GameHudState {
        GameHudState {
            health: 80.0,
            max_health: 100.0,
            ammo: 12,
            objective: "Reach the uplink",
        }
    }
}

pub struct HudDocument {
    pub html: string,
}

pub fn compose_game_hud(_state: GameHudState) -> HudDocument {
    HudDocument { html: "" }
}

pub struct FakeTime {
    pub seconds: f32,
    pub last_dt: f32,
}

impl FakeTime {
    pub fn new() -> FakeTime {
        FakeTime { seconds: 0.0, last_dt: 0.0 }
    }
    pub fn advance(self, _dt: f32) {}
    pub fn now_seconds(self) -> f32 { self.seconds }
    pub fn dt(self) -> f32 { self.last_dt }
}

pub struct FakeWorld {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl FakeWorld {
    pub fn new() -> FakeWorld {
        FakeWorld { x: 0.0, y: 0.0, z: 0.0 }
    }
    pub fn apply_forward(self, _dt: f32, _speed: f32) {}
}

pub struct FakeHud {
    pub last: HudDocument,
}

impl FakeHud {
    pub fn new() -> FakeHud {
        FakeHud { last: HudDocument { html: "" } }
    }
    pub fn present(self, document: HudDocument) {
        self.last = document
    }
}

pub struct Host {
    pub time: FakeTime,
    pub world: FakeWorld,
    pub hud: FakeHud,
}

impl Host {
    pub fn new() -> Host {
        Host {
            time: FakeTime::new(),
            world: FakeWorld::new(),
            hud: FakeHud::new(),
        }
    }
    pub fn tick_move_forward(self, dt: f32) {
        self.time.advance(dt)
        self.world.apply_forward(dt, 4.0)
        self.hud.present(compose_game_hud(GameHudState::new()))
    }
}
"#;

fn host_isolate(src: &str) -> String {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", src);
    let map = test.compile().expect("hex host compile");
    let rs = map.get("lib.rs").expect("lib.rs").clone();
    test.cargo_check().expect("hex host cargo-check");
    rs
}

fn assert_no_ffi(rs: &str) {
    assert!(
        !rs.contains("ffi::") && !rs.contains("extern fn"),
        "playable host must not call FFI:\n{rs}"
    );
}

#[test]
fn hexagonal_stub_compose_must_fail_ui_contract() {
    let rs = host_isolate(STUB);
    eprintln!("STUB lib.rs:\n{rs}");
    assert_no_ffi(&rs);
    let has_ui = rs.contains("wj-panel") && rs.contains("wj-progress");
    assert!(
        !has_ui,
        "stub isolate must stay RED (no ui classes) so the green test is meaningful:\n{rs}"
    );
}

#[test]
fn hexagonal_playable_host_must_advance_move_and_compose_ui_hud() {
    let src = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("windjammer-game/windjammer-game-core/src/application/playable_loop.wj"),
    );
    let src = match src {
        Ok(s) if s.contains("wj-panel") => s,
        _ => STUB.to_string(),
    };
    let rs = host_isolate(&src);
    eprintln!("HOST lib.rs:\n{rs}");
    assert_no_ffi(&rs);
    assert!(
        rs.contains("wj-panel") && rs.contains("wj-progress"),
        "RED: compose_game_hud must emit windjammer-ui classes:\n{rs}"
    );
    assert!(
        rs.contains("self.seconds") && (rs.contains("+= dt") || rs.contains("= self.seconds + dt") || rs.contains("= self.seconds+dt")),
        "RED: FakeTime.advance must accumulate dt:\n{rs}"
    );
    assert!(
        rs.contains("self.z") && (rs.contains("+= ") || rs.contains("= self.z +") || rs.contains("= self.z+")),
        "RED: FakeWorld.apply_forward must move z:\n{rs}"
    );
}

use std::path::PathBuf;
