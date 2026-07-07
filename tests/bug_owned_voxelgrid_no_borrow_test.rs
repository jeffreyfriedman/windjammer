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

/// Engine metadata may mark `grid: VoxelGrid` as Borrowed, but Rust FFI takes owned `VoxelGrid`.
/// Call sites must pass `grid.clone()` / owned field access — not `&grid`.
#[test]
fn test_owned_voxelgrid_param_not_auto_borrowed() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = tmp.path().join("test.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).expect("mkdir");

    fs::write(
        &wj,
        r##"
pub struct VoxelGrid {
    cells: Vec<i32>,
}

pub struct Game {
    grid: VoxelGrid,
}

impl Game {
    pub fn new() -> Game {
        Game { grid: VoxelGrid { cells: Vec::new() } }
    }

    pub fn rebuild_svo(self) {
        svo_convert(self.grid)
    }
}

pub fn svo_convert(grid: VoxelGrid) -> Vec<i32> {
    grid.cells
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

    let generated = fs::read_to_string(out.join("test.rs")).expect("test.rs");

    assert!(
        generated.contains("svo_convert(self.grid.clone())")
            || generated.contains("svo_convert(self.grid)")
            || generated.contains("svo_convert( self.grid"),
        "owned VoxelGrid param must not be passed as &grid. Generated:\n{generated}"
    );
    assert!(
        !generated.contains("svo_convert(&self.grid"),
        "must not auto-borrow owned VoxelGrid param. Generated:\n{generated}"
    );
}
