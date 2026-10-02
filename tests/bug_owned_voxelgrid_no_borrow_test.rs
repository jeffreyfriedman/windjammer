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

/// Read-only `VoxelGrid` formals may demote to `&VoxelGrid` with a matching `&`
/// call site. Forbid owned formal + `&grid` mismatch (historical E0308).
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

    let call_borrows = generated.contains("svo_convert(&self.grid");
    let formal_borrows = generated.contains("fn svo_convert(grid: &VoxelGrid)")
        || generated.contains("pub fn svo_convert(grid: &VoxelGrid)");
    assert!(
        !call_borrows || formal_borrows,
        "must not pass `&grid` into an owned VoxelGrid formal. Generated:\n{generated}"
    );

    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cargo check failed:\n{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}
