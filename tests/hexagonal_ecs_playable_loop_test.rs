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

//! Hexagonal ECS playable loop — spec 2026-09-27 Phase 2.
//! PlaytestCmd drives movement + combat + windjammer-ui HUD. No FFI.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

/// Compiles, but fire does not spend ammo or kill, and HUD has no ui classes.
const STUB: &str = r#"
pub struct PlaytestCmd {
    pub hold_forward: bool,
    pub fire_weapon: bool,
}

impl PlaytestCmd {
    pub fn idle() -> PlaytestCmd {
        PlaytestCmd { hold_forward: false, fire_weapon: false }
    }
    pub fn fire() -> PlaytestCmd {
        PlaytestCmd { hold_forward: false, fire_weapon: true }
    }
}

pub struct Enemy {
    pub health: f32,
    pub alive: bool,
}

pub struct EcsWorld {
    pub z: f32,
    pub ammo: i32,
    pub kills: i32,
    pub enemy_health: f32,
}

impl EcsWorld {
    pub fn arena() -> EcsWorld {
        EcsWorld { z: 0.0, ammo: 30, kills: 0, enemy_health: 20.0 }
    }
}

pub fn movement_system(world: EcsWorld, _dt: f32, _hold_forward: bool) {}
pub fn combat_system(world: EcsWorld, _fire: bool) {}

pub fn compose_game_hud(_ammo: i32, _kills: i32) -> string {
    ""
}

pub struct Host {
    pub world: EcsWorld,
    pub hud: string,
}

impl Host {
    pub fn new() -> Host {
        Host { world: EcsWorld::arena(), hud: "" }
    }

    pub fn tick(self, cmd: PlaytestCmd, dt: f32) {
        movement_system(self.world, dt, cmd.hold_forward)
        combat_system(self.world, cmd.fire_weapon)
        self.hud = compose_game_hud(self.world.ammo, self.world.kills)
    }
}
"#;

fn loop_isolate(src: &str) -> String {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", src);
    let map = test.compile().expect("ecs playable loop compile");
    let rs = map.get("lib.rs").expect("lib.rs").clone();
    test.cargo_check()
        .expect("ecs playable loop cargo-check");
    rs
}

fn assert_no_ffi(rs: &str) {
    assert!(
        !rs.contains("ffi::") && !rs.contains("extern fn"),
        "ECS playable loop must not call FFI:\n{rs}"
    );
}

fn product_src() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core/src/application/ecs_playable_loop.wj");
    std::fs::read_to_string(&path).unwrap_or_else(|_| STUB.to_string())
}

fn spends_ammo(rs: &str) -> bool {
    rs.contains("ammo")
        && (rs.contains("ammo -= 1")
            || rs.contains("ammo - 1")
            || rs.contains("ammo -1")
            || rs.contains("self.ammo - 1"))
}

fn increments_kills(rs: &str) -> bool {
    rs.contains("kills")
        && (rs.contains("kills += 1")
            || rs.contains("kills + 1")
            || rs.contains("self.kills + 1")
            || rs.contains("kills = kills + 1"))
}

#[test]
fn hexagonal_ecs_stub_must_stay_red_on_combat_and_hud() {
    let rs = loop_isolate(STUB);
    eprintln!("STUB ecs loop:\n{rs}");
    assert_no_ffi(&rs);
    assert!(
        !spends_ammo(&rs) && !increments_kills(&rs),
        "stub must not implement combat so the green test is meaningful:\n{rs}"
    );
    assert!(
        !rs.contains("wj-panel") && !rs.contains("wj-progress"),
        "stub HUD must stay empty:\n{rs}"
    );
}

#[test]
fn hexagonal_ecs_loop_must_fire_kill_and_compose_ui_hud() {
    let src = product_src();
    let rs = loop_isolate(&src);
    eprintln!("ECS loop lib.rs:\n{rs}");
    assert_no_ffi(&rs);
    assert!(
        spends_ammo(&rs),
        "RED: combat_system must decrement ammo on fire:\n{rs}"
    );
    assert!(
        increments_kills(&rs),
        "RED: combat_system must increment kills when an enemy dies:\n{rs}"
    );
    assert!(
        rs.contains("wj-panel") && rs.contains("wj-progress") && rs.contains("wj-text"),
        "RED: HUD must dogfood windjammer-ui classes:\n{rs}"
    );
    assert!(
        rs.contains("KILLS") || rs.contains("kills"),
        "RED: HUD / observe must surface kill count:\n{rs}"
    );
}
