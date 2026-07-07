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

/// Static impl helper with read-only `VoxelGrid` param lowers to `&VoxelGrid`. Call sites
/// that reuse a borrowed formal (e.g. camera update) must pass `grid`, not `grid.clone()`,
/// even when stale metadata marks the callee param as owned `Custom(VoxelGrid)`.
#[test]
fn test_static_readonly_voxelgrid_param_no_clone_on_reuse() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = tmp.path().join("camera.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).expect("mkdir");

    fs::write(
        &wj,
        r##"
pub struct VoxelGrid { cells: Vec<i32> }
pub struct Vec3 { x: f32, y: f32, z: f32 }
impl Vec3 { pub fn new(x: f32, y: f32, z: f32) -> Vec3 { Vec3 { x, y, z } } }

pub struct FpsCamera {}

impl FpsCamera {
    pub fn update(self, dt: f32, grid: VoxelGrid) {
        if !FpsCamera::collides_aabb(grid, Vec3::new(0.0, 0.0, 0.0), 1) {
            let _ = dt
        }
        if !FpsCamera::collides_aabb(grid, Vec3::new(1.0, 0.0, 0.0), 1) {
            let _ = dt
        }
    }

    pub fn collides_aabb(grid: VoxelGrid, pos: Vec3, scale: i32) -> bool {
        grid.cells.len() > 0
    }
}
"##,
    )
    .unwrap();

    let stub_meta = tmp.path().join("stub_meta.json");
    fs::write(
        &stub_meta,
        r##"{
  "functions": {
    "FpsCamera::collides_aabb": {
      "params": ["Custom(\"VoxelGrid\")", "Custom(\"Vec3\")", "Custom(\"i32\")"],
      "return_type": "Bool",
      "is_associated": true,
      "parent_type": "FpsCamera",
      "param_ownership": [],
      "has_self_receiver": false,
      "is_extern": false
    }
  }
}"##,
    )
    .unwrap();

    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("engine={}", stub_meta.display()),
        ])
        .output()
        .expect("wj build");

    assert!(
        build.status.success(),
        "wj build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("camera.rs")).expect("camera.rs");
    assert!(
        rs.contains("fn collides_aabb(grid: &VoxelGrid"),
        "readonly grid formal must be &VoxelGrid. Got:\n{rs}"
    );
    assert!(
        !rs.contains("collides_aabb(grid.clone()"),
        "borrowed grid formal must not be cloned for readonly static helper. Got:\n{rs}"
    );
    assert!(
        rs.contains("collides_aabb(grid,") || rs.contains("collides_aabb(&grid,"),
        "collides_aabb must receive borrowed grid. Got:\n{rs}"
    );
}

#[test]
fn test_static_readonly_voxelgrid_param_no_clone_without_metadata() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = tmp.path().join("camera.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).expect("mkdir");

    fs::write(
        &wj,
        r##"
pub struct VoxelGrid { cells: Vec<i32> }
pub struct Vec3 { x: f32, y: f32, z: f32 }
impl Vec3 { pub fn new(x: f32, y: f32, z: f32) -> Vec3 { Vec3 { x, y, z } } }

pub struct FpsCamera {}

impl FpsCamera {
    pub fn update(self, dt: f32, grid: VoxelGrid) {
        if !FpsCamera::collides_aabb(grid, Vec3::new(0.0, 0.0, 0.0), 1) {
            let _ = dt
        }
        if !FpsCamera::collides_aabb(grid, Vec3::new(1.0, 0.0, 0.0), 1) {
            let _ = dt
        }
    }

    pub fn collides_aabb(grid: VoxelGrid, pos: Vec3, scale: i32) -> bool {
        grid.cells.len() > 0
    }
}
"##,
    )
    .unwrap();

    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");

    assert!(
        build.status.success(),
        "wj build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("camera.rs")).expect("camera.rs");
    assert!(
        !rs.contains("collides_aabb(grid.clone()"),
        "must not clone borrowed grid formal without metadata either. Got:\n{rs}"
    );
}
