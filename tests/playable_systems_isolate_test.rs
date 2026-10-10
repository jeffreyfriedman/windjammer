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

//! ECS playable systems isolate — fire/move/HUD without FFI.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const STUB: &str = r#"
pub struct PlayableWorld {
    pub z: f32,
    pub ammo: i32,
    pub kills: i32,
    pub enemy_health: f32,
}
impl PlayableWorld {
    pub fn arena() -> PlayableWorld {
        PlayableWorld { z: 0.0, ammo: 30, kills: 0, enemy_health: 20.0 }
    }
}
pub fn movement_system(_world: PlayableWorld, _dt: f32, _hold_forward: bool) {}
pub fn combat_system(_world: PlayableWorld, _fire: bool) {}
pub fn compose_status_hud(_world: PlayableWorld) -> string { "" }
pub fn tick_playable(world: PlayableWorld, hold_forward: bool, fire: bool, dt: f32) -> string {
    movement_system(world, dt, hold_forward)
    combat_system(world, fire)
    compose_status_hud(world)
}
"#;

fn isolate_compile(src: &str) -> String {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", src);
    let map = test.compile().expect("bp systems compile");
    map.get("lib.rs").expect("lib.rs").clone()
}

#[test]
fn playable_systems_stub_stays_red() {
    // Compile-only: avoid parallel cargo-check lock contention with the green test.
    let rs = isolate_compile(STUB);
    assert!(!rs.contains("wj-panel"));
    assert!(!rs.contains("ammo -= 1") && !rs.contains("ammo - 1"));
}
