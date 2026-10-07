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

//! P3.707: Breach Protocol `BreachProtocolGame::update_death_state` mutates
//! `self.respawn_timer` / camera / renderer HUD but tip emit keeps owned
//! immutable `self` (`error[E0594]: cannot assign … as self is not declared
//! as mutable`).
//!
//! Product shape: field assigns + `player.is_alive` / `player.respawn` + call to
//! owned-self `upload_camera` that partial-moves camera fields. Minimal
//! field-assign-only isolates stay GREEN; this MultiFile matches the RED product.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC_MOD: &str = r#"
pub mod game
pub mod camera
pub mod state
"#;

const SRC_GAME: &str = r#"
pub struct Cam { pub x: f32 }
pub struct Player { pub health: f32, pub alive: bool }
pub struct Renderer { pub id: i32 }
pub struct Game {
    pub player: Player,
    pub respawn_timer: f32,
    pub camera: Cam,
    pub tps_camera: Cam,
    pub renderer: Renderer,
    pub last_dt: f32,
    pub damage_flash_timer: f32,
    pub death_count: i32,
    pub component_test_id: i32,
    pub camera_mode: i32,
    pub kills_tracked: i32,
}

impl Player {
    pub fn is_alive(self) -> bool { self.alive }
    pub fn respawn(self) {
        self.alive = true
        self.health = 100.0
    }
}

impl Renderer {
    pub fn get_voxel_renderer(self) -> i32 { self.id }
    pub fn set_hud_state(self, h: f32, mh: f32, a: f32, ma: f32, x: f32, e: f32, me: f32) {
        self.id = self.id
    }
    pub fn set_kill_count(self, k: i32) { self.id = k }
    pub fn set_death_timer(self, t: f32) { self.id = self.id }
}

pub fn take_upload(vr: i32, c: Cam, tps: Cam, id: i32, mode: i32) {}
"#;

const SRC_CAMERA: &str = r#"
use crate::game::Game
use crate::game::take_upload

impl Game {
    fn upload_camera(self) {
        take_upload(
            self.renderer.get_voxel_renderer(), self.camera, self.tps_camera,
            self.component_test_id, self.camera_mode)
    }
}
"#;

const SRC_STATE: &str = r#"
use crate::game::Game
use crate::game::Cam

impl Game {
    fn update_death_state(self, dt: f32) -> bool {
        if !self.player.is_alive() {
            self.respawn_timer = self.respawn_timer - dt
            if self.respawn_timer <= 0.0 {
                self.player.respawn()
                self.camera = Cam { x: 32.0 }
                self.upload_camera()
                self.respawn_timer = 0.0
                println!("deaths {}", self.death_count)
            } else {
                self.damage_flash_timer = 0.15
            }
            self.renderer.set_hud_state(
                self.player.health, self.player.health,
                0.0, 0.0, 1.0, 0.0, 0.0)
            self.renderer.set_kill_count(self.kills_tracked)
            self.renderer.set_death_timer(self.respawn_timer)
            self.last_dt = dt
            return true
        }
        false
    }
}
"#;

#[test]
fn bp_module_file_update_death_state_must_emit_mut_self() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", SRC_MOD);
    test.add_file("game.wj", SRC_GAME);
    test.add_file("camera.wj", SRC_CAMERA);
    test.add_file("state.wj", SRC_STATE);
    let map = test.compile().expect("P3.707 compile");
    let rs = map.get("state.rs").expect("state.rs");
    eprintln!("P3.707 MultiFile state.rs:\n{rs}");
    let ok = rs.contains("fn update_death_state(&mut self")
        || rs.contains("fn update_death_state(mut self");
    assert!(
        ok,
        "P3.707 RED: update_death_state must emit &mut self or mut self \
         (field assigns + owned upload_camera), not bare self:\n{rs}"
    );
}

#[test]
fn bp_tip_out_update_death_state_must_emit_mut_self() {
    let mut paths = Vec::new();
    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let root = PathBuf::from(dir);
        paths.push(
            root.parent()
                .unwrap_or(root.as_path())
                .join("breach-protocol/gen/game_state.rs"),
        );
        paths.push(
            root.parent()
                .unwrap_or(root.as_path())
                .join("breach-protocol/build/game_state.rs"),
        );
    }
    paths.push(PathBuf::from(
        "/Users/jeffreyfriedman/src/wj/breach-protocol/gen/game_state.rs",
    ));
    // Fresh tip multipass (agents often land under /tmp).
    paths.push(PathBuf::from("/tmp/wj_bp_tip_full/game_state.rs"));

    let path = paths.into_iter().find(|p| p.exists());
    let Some(path) = path else {
        eprintln!("skip: breach-protocol gen/game_state.rs not present");
        return;
    };
    let rs = std::fs::read_to_string(&path).expect("read game_state.rs");
    assert!(
        rs.contains("fn update_death_state(&mut self")
            || rs.contains("fn update_death_state(mut self"),
        "P3.707 tip-out: update_death_state must emit &mut self (or mut self), not owned immutable self. path={} snippet:\n{}",
        path.display(),
        rs.lines()
            .filter(|l| l.contains("update_death_state") || l.contains("respawn_timer"))
            .take(12)
            .collect::<Vec<_>>()
            .join("\n")
    );
}
