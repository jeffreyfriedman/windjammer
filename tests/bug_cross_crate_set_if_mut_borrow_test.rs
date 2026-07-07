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

fn build_engine(tmp: &TempDir) -> std::path::PathBuf {
    let engine_src = tmp.path().join("engine_src");
    let engine_gen = tmp.path().join("engine_gen");
    fs::create_dir_all(engine_src.join("scene")).expect("mkdir");
    fs::create_dir_all(engine_src.join("voxel")).expect("mkdir");

    fs::write(engine_src.join("mod.wj"), "mod scene\nmod voxel\n").unwrap();
    fs::write(engine_src.join("voxel/mod.wj"), "mod grid\n").unwrap();
    fs::write(
        engine_src.join("voxel/grid.wj"),
        r##"
pub struct VoxelGrid {
    cells: Vec<i32>,
}

impl VoxelGrid {
    pub fn new(w: i32, h: i32, d: i32) -> VoxelGrid {
        VoxelGrid { cells: Vec::new() }
    }

    pub fn set(self, x: i32, y: i32, z: i32, v: i32) {
        self.cells.push(x + y + z + v)
    }
}
"##,
    )
    .unwrap();
    fs::write(engine_src.join("scene/mod.wj"), "mod station_builder\n").unwrap();
    fs::write(
        engine_src.join("scene/station_builder.wj"),
        r##"
use engine::voxel::grid::VoxelGrid

pub fn set_if(grid: VoxelGrid, x: i32, y: i32, z: i32, mat: i32) {
    grid.set(x, y, z, mat)
}
"##,
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            engine_src.to_str().unwrap(),
            "--output",
            engine_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .status()
        .expect("engine build");
    assert!(status.success(), "engine library build failed");
    engine_gen.join("metadata.json")
}

fn compile_scene_builder(tmp: &TempDir, engine_meta: &std::path::Path) -> String {
    let game_src = tmp.path().join("game_src");
    fs::create_dir_all(&game_src).expect("mkdir");
    fs::write(
        game_src.join("scene_builder.wj"),
        r##"
use engine::voxel::grid::VoxelGrid
use engine::scene::station_builder

fn fill_hull(grid: VoxelGrid, logical_size: i32) {
    let mut x = 0
    while x < logical_size {
        station_builder::set_if(grid, x, 0, 0, 1)
        x = x + 1
    }
}

fn station_builder_carve_room(grid: VoxelGrid, x0: i32, z0: i32, x1: i32, z1: i32, floor_y: i32, ceil_y: i32, floor_mat: i32) {
    station_builder::set_if(grid, x0, floor_y, z0, floor_mat)
}
"##,
    )
    .unwrap();

    let game_gen = tmp.path().join("game_gen");
    let status = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            game_src.join("scene_builder.wj").to_str().unwrap(),
            "--output",
            game_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("engine={}", engine_meta.display()),
        ])
        .status()
        .expect("game build");
    assert!(status.success(), "game build failed");
    fs::read_to_string(game_gen.join("scene_builder.rs")).expect("scene_builder.rs")
}

fn assert_mut_voxelgrid_passthrough(rs: &str) {
    assert!(
        rs.contains("fn fill_hull(grid: &mut VoxelGrid")
            || rs.contains("fn fill_hull(mut grid: VoxelGrid"),
        "fill_hull grid param must be mutably borrowed. Got:\n{rs}"
    );
    assert!(
        !rs.contains("fn fill_hull(grid: &VoxelGrid,")
            && !rs.contains("fn fill_hull(grid: &VoxelGrid)"),
        "immutable &VoxelGrid formal breaks mutating set_if calls. Got:\n{rs}"
    );
    assert!(
        !rs.contains("set_if(grid.clone()"),
        "set_if must not receive an owned clone of grid. Got:\n{rs}"
    );
    assert!(
        rs.contains("station_builder::set_if(grid,")
            || rs.contains("station_builder::set_if(&mut grid,"),
        "set_if must receive grid by mutable reborrow. Got:\n{rs}"
    );
}

/// Breach-protocol scene_builder.wj: VoxelGrid from engine::voxel::grid, mutating via
/// engine::scene::station_builder::set_if must infer &mut formals and pass &mut at calls.
#[test]
fn test_cross_crate_set_if_mut_borrow_with_split_voxelgrid_import() {
    let tmp = TempDir::new().expect("tempdir");
    let engine_meta = build_engine(&tmp);
    let rs = compile_scene_builder(&tmp, &engine_meta);
    assert_mut_voxelgrid_passthrough(&rs);
}

/// Real engine metadata omits many module-level helpers like `set_if`. Without registry
/// entries, cross-crate calls must still infer &mut for the first non-Copy argument.
#[test]
fn test_cross_crate_set_if_mut_borrow_when_callee_missing_from_metadata() {
    let tmp = TempDir::new().expect("tempdir");
    let engine_meta = build_engine(&tmp);
    let stripped_meta = tmp.path().join("metadata_stripped.json");
    let raw = fs::read_to_string(&engine_meta).expect("read metadata");
    let mut meta: serde_json::Value = serde_json::from_str(&raw).expect("parse metadata");
    if let Some(funcs) = meta.get_mut("functions").and_then(|v| v.as_object_mut()) {
        funcs.retain(|k, _| !k.contains("set_if"));
    }
    fs::write(&stripped_meta, meta.to_string()).expect("write stripped metadata");

    let rs = compile_scene_builder(&tmp, &stripped_meta);
    assert_mut_voxelgrid_passthrough(&rs);
}
