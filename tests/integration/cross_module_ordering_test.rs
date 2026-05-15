//! TDD Test: Verify cross-module ownership works regardless of file ordering
//!
//! Tests that signature resolution works even when the callee is defined
//! in a file alphabetically AFTER the caller (compilation order dependency).

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_callee_defined_after_caller_alphabetically() {
    // "alpha.wj" calls "zeta.wj" — zeta comes after alpha alphabetically
    // Ensures multi-pass analysis resolves signatures correctly
    let files = &[
        (
            "alpha.wj",
            r#"
use crate::Grid
use crate::zeta

pub struct Game {
    pub grid: Grid,
}

impl Game {
    pub fn update(self) {
        zeta::fill(self.grid)
    }
}
"#,
        ),
        (
            "zeta.wj",
            r#"
pub struct Grid {
    pub data: Vec<i32>,
}

impl Grid {
    pub fn new() -> Grid { Grid { data: Vec::new() } }
    pub fn set(self, val: i32) { self.data.push(val) }
}

pub fn fill(grid: Grid) {
    grid.set(42)
}
"#,
        ),
    ];

    let results = test_utils::compile_project_result(files).expect("Should compile");
    let alpha_rs = results
        .get("alpha.rs")
        .expect("alpha.rs should be generated");

    assert!(
        alpha_rs.contains("pub fn update(&mut self"),
        "update() should infer &mut self even when callee is defined in later file. Got:\n{}",
        alpha_rs
    );
}

#[test]
fn test_deeply_nested_cross_module_chain() {
    // A -> B -> C: Game.update() calls module_b::process() which calls module_c::mutate()
    // The mutation in C should propagate up through B to A
    // State must be non-Copy (has Vec field) so passthrough inference is exercised
    let files = &[
        (
            "game.wj",
            r#"
use crate::State
use crate::module_b

pub struct Game {
    pub state: State,
}

impl Game {
    pub fn update(self) {
        module_b::process(self.state)
    }
}
"#,
        ),
        (
            "module_b.wj",
            r#"
use crate::State
use crate::module_c

pub fn process(state: State) {
    module_c::mutate(state)
}
"#,
        ),
        (
            "module_c.wj",
            r#"
pub struct State {
    pub items: Vec<i32>,
}

pub fn mutate(state: State) {
    state.items.push(1)
}
"#,
        ),
    ];

    let results = test_utils::compile_project_result(files).expect("Should compile");
    let game_rs = results.get("game.rs").expect("game.rs should be generated");
    let b_rs = results
        .get("module_b.rs")
        .expect("module_b.rs should be generated");

    assert!(
        b_rs.contains("state: &mut State"),
        "process() should infer &mut State from passthrough to module_c::mutate. Got:\n{}",
        b_rs
    );
    assert!(
        game_rs.contains("pub fn update(&mut self"),
        "Game.update() should infer &mut self from transitive mutation chain. Got:\n{}",
        game_rs
    );
}
