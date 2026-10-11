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

//! Copy `const i32` into owned call formals must not `.clone()`.
//!
//! Product `scene/station_builder.rs`:
//!   `VoxelGrid::new(GRID.clone(), GRID.clone(), GRID.clone())`
//! WJ is `VoxelGrid::new(GRID, GRID, GRID)`.
//! Distinct from (`LEAF_FLAG.clone()` on **assign**), (local in
//! println), and (`f32::MAX.clone()` assoc path).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub const GRID: i32 = 64

pub struct VoxelGrid {
    pub w: i32,
    pub h: i32,
    pub d: i32,
}

impl VoxelGrid {
    pub fn new(w: i32, h: i32, d: i32) -> VoxelGrid {
        VoxelGrid { w: w, h: h, d: d }
    }
}

pub fn make_grid() -> VoxelGrid {
    VoxelGrid::new(GRID, GRID, GRID)
}
"#;

#[test]
fn module_file_const_i32_into_call_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("GRID.clone()");
    assert!(
        !bad,
        "const i32 cloned into call formals:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
