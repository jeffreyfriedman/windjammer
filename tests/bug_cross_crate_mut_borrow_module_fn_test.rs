#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Cross-crate module-qualified free fn: engine metadata `MutBorrowed` grid param must
/// survive declaration-stub merge and reach game call sites as `&mut grid`, not `grid.clone()`.
#[test]
fn test_cross_crate_module_fn_mut_borrow_from_engine_metadata() {
    let tmp = TempDir::new().expect("tempdir");

    let engine_src = tmp.path().join("engine_src");
    let engine_gen = tmp.path().join("engine_gen");
    let scene_dir = engine_src.join("scene");
    fs::create_dir_all(&scene_dir).expect("mkdir");

    fs::write(
        scene_dir.join("mod.wj"),
        r#"pub mod geometry
pub mod builder
"#,
    )
    .unwrap();

    fs::write(
        scene_dir.join("geometry.wj"),
        r##"
pub struct Grid {
    cells: Vec<i32>,
}

impl Grid {
    pub fn mark(self, x: i32) {
        self.cells.push(x)
    }
}

pub fn touch_grid(grid: Grid, x: i32) {
    grid.mark(x)
}
"##,
    )
    .unwrap();

    fs::write(
        scene_dir.join("builder.wj"),
        r#"pub use crate::scene::geometry::{Grid, touch_grid}
"#,
    )
    .unwrap();

    let engine_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            engine_src.to_str().unwrap(),
            "--output",
            engine_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("engine build");

    assert!(
        engine_build.status.success(),
        "engine build failed:\n{}",
        String::from_utf8_lossy(&engine_build.stderr)
    );

    let engine_meta = engine_gen.join("metadata.json");
    assert!(engine_meta.exists(), "engine must emit metadata.json");

    let game_src = tmp.path().join("game_src");
    fs::create_dir_all(&game_src).expect("mkdir");

    fs::write(
        game_src.join("game.wj"),
        r##"
use engine::scene::builder::{Grid, touch_grid}

pub fn build(mut grid: Grid) {
    let mut i = 0
    while i < 3 {
        touch_grid(grid, i)
        i = i + 1
    }
}
"##,
    )
    .unwrap();

    let game_gen = tmp.path().join("game_gen");
    let game_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            game_src.join("game.wj").to_str().unwrap(),
            "--output",
            game_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("engine={}", engine_meta.display()),
        ])
        .output()
        .expect("game build");

    assert!(
        game_build.status.success(),
        "game build failed:\n{}",
        String::from_utf8_lossy(&game_build.stderr)
    );

    let generated = fs::read_to_string(game_gen.join("game.rs")).expect("game.rs");

    assert!(
        generated.contains("touch_grid(&mut grid,") || generated.contains("touch_grid( &mut grid,"),
        "cross-crate module fn must pass &mut grid from engine metadata. Generated:\n{generated}"
    );
    assert!(
        !generated.contains("touch_grid(grid.clone()"),
        "must not clone grid for &mut param. Generated:\n{generated}"
    );
}

/// When the caller's parameter is already inferred as `&mut T`, passing it to another
/// `&mut T` callee must not emit `&mut param` (illegal `&mut &mut T`, E0596).
#[test]
fn test_cross_crate_no_double_mut_when_param_already_mut_borrowed() {
    let tmp = TempDir::new().expect("tempdir");

    let engine_src = tmp.path().join("engine_src");
    let engine_gen = tmp.path().join("engine_gen");
    let scene_dir = engine_src.join("scene");
    fs::create_dir_all(&scene_dir).expect("mkdir");

    fs::write(
        scene_dir.join("mod.wj"),
        r#"pub mod geometry
pub mod builder
"#,
    )
    .unwrap();

    fs::write(
        scene_dir.join("geometry.wj"),
        r##"
pub struct Grid {
    cells: Vec<i32>,
}

impl Grid {
    pub fn mark(self, x: i32) {
        self.cells.push(x)
    }
}

pub fn touch_grid(grid: Grid, x: i32) {
    grid.mark(x)
}
"##,
    )
    .unwrap();

    fs::write(
        scene_dir.join("builder.wj"),
        r#"pub use crate::scene::geometry::{Grid, touch_grid}
"#,
    )
    .unwrap();

    let engine_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            engine_src.to_str().unwrap(),
            "--output",
            engine_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("engine build");

    assert!(
        engine_build.status.success(),
        "engine build failed:\n{}",
        String::from_utf8_lossy(&engine_build.stderr)
    );

    let engine_meta = engine_gen.join("metadata.json");

    let game_src = tmp.path().join("game_src");
    fs::create_dir_all(&game_src).expect("mkdir");

    fs::write(
        game_src.join("game.wj"),
        r##"
use engine::scene::builder::{Grid, touch_grid}

pub fn fill_hull(grid: Grid) {
    touch_grid(grid, 0)
    touch_grid(grid, 1)
}
"##,
    )
    .unwrap();

    let game_gen = tmp.path().join("game_gen");
    let game_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            game_src.join("game.wj").to_str().unwrap(),
            "--output",
            game_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("engine={}", engine_meta.display()),
        ])
        .output()
        .expect("game build");

    assert!(
        game_build.status.success(),
        "game build failed:\n{}",
        String::from_utf8_lossy(&game_build.stderr)
    );

    let generated = fs::read_to_string(game_gen.join("game.rs")).expect("game.rs");

    assert!(
        generated.contains("fn fill_hull(grid: &mut Grid"),
        "fill_hull grid param should be &mut Grid. Generated:\n{generated}"
    );
    assert!(
        !generated.contains("&mut grid"),
        "must not double-borrow &mut param when passing to another &mut callee. Generated:\n{generated}"
    );
    assert!(
        generated.contains("touch_grid(grid,") || generated.contains("touch_grid( grid,"),
        "should pass grid directly (reborrow). Generated:\n{generated}"
    );
}
