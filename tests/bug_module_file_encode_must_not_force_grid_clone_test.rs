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

//! WDB-360: `encode(grid)` must not require `grid.clone()` when encode only reads.
//!
//! Tip output for a game project cluster:
//!   `encoder.encode(self.grid.clone())` / `encode(self.chunks[i].clone().grid.clone())`
//!   with `encode(&mut self, grid: VoxelGrid)` owned formal forcing a clone of a large grid.
//! Prefer borrowed formal `encode(grid: &VoxelGrid)` + call `encode(&self.grid)`.
//! Twin polarity of WDB-335 (owned clone into demoted `&VoxelGrid`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct VoxelGrid {
    pub w: i32,
}

pub struct Encoder {
    pub n: i32,
}

impl Encoder {
    pub fn encode(self, grid: VoxelGrid) -> Vec<i32> {
        let mut out: Vec<i32> = Vec::new()
        out.push(grid.w)
        out
    }
}

pub struct Scene {
    pub grid: VoxelGrid,
}

pub fn bake(scene: Scene) -> Vec<i32> {
    let mut enc = Encoder { n: 0 }
    enc.encode(scene.grid)
}
"#;

#[test]
fn module_file_encode_must_not_force_grid_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    // After fix: encode should take &VoxelGrid and call sites borrow — no .clone() on grid.
    let bad = rs.contains("grid.clone()") || rs.contains(".grid.clone()");
    assert!(
        !bad,
        "encode path cloned grid:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
