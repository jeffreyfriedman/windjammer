//! TDD Test: Cross-module ownership inference for self.field passed to free functions
//!
//! Bug: When a method calls a module-qualified free function (e.g., helpers::fill_grid(self.grid)),
//! the self_analysis doesn't detect transitive mutation because the signature lookup uses the
//! full qualified name "helpers::fill_grid" but the registry stores it as "fill_grid".
//!
//! Result: The method gets &self instead of &mut self, causing rustc errors:
//! - E0596: cannot borrow `self.grid` as mutable, as it is behind a `&` reference
//! - E0507: cannot move out of `self.grid` which is behind a shared reference
//!
//! Root Cause: self_analysis.rs call_function_name() returns "helpers::fill_grid" but
//! reg.get_signature() only has "fill_grid" — no fallback to try the base name.
//! The codegen already has this fallback (expression_generation.rs lines 2238-2261),
//! but self_analysis.rs and passthrough_inference.rs do not.
//!
//! Fix: Add base-name fallback to self_analysis.rs signature lookups.

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_cross_module_free_fn_mutates_self_field() {
    // Scenario: helpers::fill_grid(self.grid) where fill_grid mutates grid
    // Expected: update() infers &mut self because self.grid is passed to &mut parameter
    let code = r#"
pub struct VoxelGrid {
    pub data: Vec<i32>,
}

impl VoxelGrid {
    pub fn new() -> VoxelGrid {
        VoxelGrid { data: Vec::new() }
    }
    pub fn set(self, index: i32, value: i32) {
        self.data.push(value)
    }
}

pub fn fill_grid(grid: VoxelGrid) {
    grid.set(0, 42)
}

pub struct Game {
    pub grid: VoxelGrid,
}

impl Game {
    pub fn update(self) {
        fill_grid(self.grid)
    }
}
"#;

    let rust = test_utils::compile_single_result(code).expect("Should compile");

    // Baseline: unqualified call should infer &mut self
    assert!(
        rust.contains("pub fn update(&mut self"),
        "update() with unqualified fill_grid(self.grid) should infer &mut self. Generated:\n{}",
        rust
    );
    test_utils::verify_rust_compiles(&rust).expect("Generated Rust should compile");
}

#[test]
fn test_cross_module_qualified_call_mutates_self_field() {
    // The ACTUAL bug: module-qualified call helpers::fill_grid(self.grid)
    // The parser produces Identifier("helpers::fill_grid") but the registry
    // stores "fill_grid" — lookup fails, &mut self not detected.
    let files = &[
        (
            "helpers.wj",
            r#"
pub struct VoxelGrid {
    pub data: Vec<i32>,
}

impl VoxelGrid {
    pub fn new() -> VoxelGrid {
        VoxelGrid { data: Vec::new() }
    }
    pub fn set(self, index: i32, value: i32) {
        self.data.push(value)
    }
}

pub fn fill_grid(grid: VoxelGrid) {
    grid.set(0, 42)
}
"#,
        ),
        (
            "game.wj",
            r#"
use crate::VoxelGrid
use crate::helpers

pub struct Game {
    pub grid: VoxelGrid,
}

impl Game {
    pub fn update(self) {
        helpers::fill_grid(self.grid)
    }
}
"#,
        ),
    ];

    let results = test_utils::compile_project_result(files).expect("Should compile");

    let game_rs = results.get("game.rs").expect("game.rs should be generated");

    assert!(
        game_rs.contains("pub fn update(&mut self"),
        "update() with qualified helpers::fill_grid(self.grid) should infer &mut self.\n\
         Bug: self_analysis doesn't resolve module-qualified names in signature registry.\n\
         Generated game.rs:\n{}",
        game_rs
    );
}

#[test]
fn test_cross_module_qualified_call_readonly_stays_borrowed() {
    // Negative test: readonly cross-module call should NOT upgrade to &mut self
    let files = &[
        (
            "helpers.wj",
            r#"
pub struct VoxelGrid {
    pub data: Vec<i32>,
}

impl VoxelGrid {
    pub fn new() -> VoxelGrid {
        VoxelGrid { data: Vec::new() }
    }
}

pub fn count_voxels(grid: VoxelGrid) -> i32 {
    grid.data.len()
}
"#,
        ),
        (
            "game.wj",
            r#"
use crate::VoxelGrid
use crate::helpers

pub struct Game {
    pub grid: VoxelGrid,
}

impl Game {
    pub fn voxel_count(self) -> i32 {
        helpers::count_voxels(self.grid)
    }
}
"#,
        ),
    ];

    let results = test_utils::compile_project_result(files).expect("Should compile");
    let game_rs = results.get("game.rs").expect("game.rs should be generated");

    assert!(
        game_rs.contains("pub fn voxel_count(&self") || game_rs.contains("pub fn voxel_count(self"),
        "voxel_count() with readonly helpers::count_voxels(self.grid) should NOT be &mut self.\n\
         Generated game.rs:\n{}",
        game_rs
    );
}

#[test]
fn test_cross_module_multiple_fields_mixed_ownership() {
    // Multiple self.fields passed to cross-module function, some &mut, some &
    let files = &[
        (
            "helpers.wj",
            r#"
pub struct World {
    pub data: Vec<i32>,
}

impl World {
    pub fn new() -> World {
        World { data: Vec::new() }
    }
    pub fn add(self, val: i32) {
        self.data.push(val)
    }
}

pub fn spawn_entity(world: World, count: i32) {
    world.add(count)
}
"#,
        ),
        (
            "game.wj",
            r#"
use crate::World
use crate::helpers

pub struct Game {
    pub world: World,
    pub entity_count: i32,
}

impl Game {
    pub fn spawn(self) {
        helpers::spawn_entity(self.world, self.entity_count)
    }
}
"#,
        ),
    ];

    let results = test_utils::compile_project_result(files).expect("Should compile");
    let game_rs = results.get("game.rs").expect("game.rs should be generated");

    assert!(
        game_rs.contains("pub fn spawn(&mut self"),
        "spawn() should infer &mut self because self.world is passed as &mut.\n\
         Generated game.rs:\n{}",
        game_rs
    );
}

#[test]
fn test_passthrough_ownership_cross_module() {
    // passthrough_inference.rs should also resolve cross-module function signatures
    // when determining parameter ownership for free functions that forward to module calls
    let files = &[
        (
            "core.wj",
            r#"
pub struct Buffer {
    pub items: Vec<i32>,
}

impl Buffer {
    pub fn new() -> Buffer {
        Buffer { items: Vec::new() }
    }
    pub fn append(self, val: i32) {
        self.items.push(val)
    }
}

pub fn process_buffer(buf: Buffer, val: i32) {
    buf.append(val)
}
"#,
        ),
        (
            "app.wj",
            r#"
use crate::Buffer
use crate::core

pub fn run(buf: Buffer) {
    core::process_buffer(buf, 42)
}
"#,
        ),
    ];

    let results = test_utils::compile_project_result(files).expect("Should compile");
    let app_rs = results.get("app.rs").expect("app.rs should be generated");

    assert!(
        app_rs.contains("buf: &mut Buffer"),
        "run() parameter buf should infer &mut Buffer from passthrough to core::process_buffer.\n\
         Generated app.rs:\n{}",
        app_rs
    );
}
