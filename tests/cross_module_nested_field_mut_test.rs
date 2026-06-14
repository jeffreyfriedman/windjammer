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
))]

//! TDD Test: Cross-module ownership for complex nested field patterns
//!
//! Tests the EXACT pattern that blocked refactoring a large game struct:
//! - Struct with multiple non-Copy fields
//! - Free function that mutates sub-fields (player.health = ...)
//! - Multiple self.field arguments to one function call
//! - Nested struct mutation through method + field chain

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_free_fn_subfield_mutation() {
    // Pattern: free function mutates a sub-field of its parameter
    // fn update_health(player: PlayerState) { player.health = player.health + 1.0 }
    // Called as: update_health(self.player)
    // Expected: update() needs &mut self, player param needs &mut
    let code = r#"
pub struct PlayerState {
    pub health: f32,
    pub max_health: f32,
}

pub fn update_health_regen(player: PlayerState, delta: f32) {
    if player.health < player.max_health {
        player.health = player.health + delta * 10.0
    }
}

pub struct Game {
    pub player: PlayerState,
}

impl Game {
    pub fn update(self, delta: f32) {
        update_health_regen(self.player, delta)
    }
}
"#;

    let rust = test_utils::compile_single_result(code).expect("Should compile");
    assert!(
        rust.contains("pub fn update(&mut self"),
        "update() should infer &mut self when self.player passed to mutating free fn. Got:\n{}",
        rust
    );
    assert!(
        rust.contains("player: &mut PlayerState"),
        "update_health_regen should infer &mut PlayerState. Got:\n{}",
        rust
    );
    test_utils::verify_rust_compiles(&rust).expect("Generated Rust should compile");
}

#[test]
fn test_cross_module_subfield_mutation() {
    // Same pattern but cross-module (the actual bug scenario)
    let files = &[
        (
            "game_helpers.wj",
            r#"
pub struct PlayerState {
    pub health: f32,
    pub max_health: f32,
}

pub fn update_health_regen(player: PlayerState, delta: f32) {
    if player.health < player.max_health {
        player.health = player.health + delta * 10.0
    }
}
"#,
        ),
        (
            "game.wj",
            r#"
use crate::PlayerState
use crate::game_helpers

pub struct Game {
    pub player: PlayerState,
}

impl Game {
    pub fn update(self, delta: f32) {
        game_helpers::update_health_regen(self.player, delta)
    }
}
"#,
        ),
    ];

    let results = test_utils::compile_project_result(files).expect("Should compile");
    let game_rs = results.get("game.rs").expect("game.rs should be generated");
    let helpers_rs = results
        .get("game_helpers.rs")
        .expect("game_helpers.rs should be generated");

    assert!(
        helpers_rs.contains("player: &mut PlayerState"),
        "update_health_regen should infer &mut PlayerState from sub-field mutation. Got:\n{}",
        helpers_rs
    );

    assert!(
        game_rs.contains("pub fn update(&mut self"),
        "update() should infer &mut self when self.player passed to cross-module &mut fn. Got:\n{}",
        game_rs
    );
}

#[test]
fn test_multiple_self_fields_to_cross_module_fn() {
    // Pattern: multiple self.fields passed to a free function
    // Pattern: spawn_next_wave(self.grid, self.enemies, self.wave)
    let files = &[
        (
            "spawning.wj",
            r#"
pub struct VoxelGrid {
    pub cells: Vec<i32>,
}

impl VoxelGrid {
    pub fn new() -> VoxelGrid { VoxelGrid { cells: Vec::new() } }
    pub fn set(self, idx: i32, val: i32) { self.cells.push(val) }
}

pub struct Enemy {
    pub hp: i32,
}

pub fn spawn_next_wave(grid: VoxelGrid, enemies: Vec<Enemy>, wave: i32) {
    grid.set(wave, 1)
    enemies.push(Enemy { hp: 100 })
}
"#,
        ),
        (
            "entry.wj",
            r#"
use crate::VoxelGrid
use crate::Enemy
use crate::spawning

pub struct MainApp {
    pub grid: VoxelGrid,
    pub enemies: Vec<Enemy>,
    pub wave: i32,
}

impl MainApp {
    pub fn update(self) {
        spawning::spawn_next_wave(self.grid, self.enemies, self.wave)
    }
}
"#,
        ),
    ];

    let results = test_utils::compile_project_result(files).expect("Should compile");
    let entry_rs = results
        .get("entry.rs")
        .expect("entry.rs should be generated");

    assert!(
        entry_rs.contains("pub fn update(&mut self"),
        "update() should infer &mut self when self.grid and self.enemies passed to mutating fn. Got:\n{}",
        entry_rs
    );
}

#[test]
fn test_nested_method_then_cross_module_call() {
    // Pattern: private method calls cross-module function
    // update() -> self.do_spawn() -> spawning::spawn(self.grid)
    // Two levels of transitive mutation
    let files = &[
        (
            "spawning.wj",
            r#"
pub struct Grid {
    pub data: Vec<i32>,
}

impl Grid {
    pub fn new() -> Grid { Grid { data: Vec::new() } }
    pub fn add(self, val: i32) { self.data.push(val) }
}

pub fn spawn(grid: Grid) {
    grid.add(1)
}
"#,
        ),
        (
            "game.wj",
            r#"
use crate::Grid
use crate::spawning

pub struct Game {
    pub grid: Grid,
    pub score: i32,
}

impl Game {
    pub fn update(self) {
        self.do_spawn()
        self.score = self.score + 1
    }

    fn do_spawn(self) {
        spawning::spawn(self.grid)
    }
}
"#,
        ),
    ];

    let results = test_utils::compile_project_result(files).expect("Should compile");
    let game_rs = results.get("game.rs").expect("game.rs should be generated");

    assert!(
        game_rs.contains("fn do_spawn(&mut self"),
        "do_spawn() should infer &mut self from spawning::spawn(self.grid). Got:\n{}",
        game_rs
    );
    assert!(
        game_rs.contains("pub fn update(&mut self"),
        "update() should infer &mut self from self.do_spawn() + self.score mutation. Got:\n{}",
        game_rs
    );
}
