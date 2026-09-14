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

//! Engine tip blocker (windjammer-game-core `component_viewer_controls.wj`):
//!
//!   `grid.set(*x.clone(), *y.clone(), z, mat as u8)` → E0614
//!   (`type i32 cannot be dereferenced`)
//!
//! Owned Copy `i32` formals passed into owned Copy method formals must emit
//! bare `x` / `y` (Rust auto-copies). Never `*x.clone()` or `*x`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod grid
pub mod viewer
"#;

const GRID: &str = r#"
pub struct VoxelGrid {
    pub w: i32,
}

impl VoxelGrid {
    pub fn new(w: i32) -> VoxelGrid {
        VoxelGrid { w: w }
    }

    pub fn set(self, x: i32, y: i32, z: i32, value: u8) {
        let _ = x
        let _ = y
        let _ = z
        let _ = value
    }
}
"#;

const VIEWER: &str = r#"
use crate::grid::VoxelGrid

const VIEWER_GRID: i32 = 64

fn set_if(grid: VoxelGrid, x: i32, y: i32, z: i32, mat: i32) {
    // Multi-use Copy formals (bounds check + call) — product shape from
    // component_viewer_controls.wj that tip-gen'd as `*x.clone()`.
    if x >= 0 && x < VIEWER_GRID && y >= 0 && y < VIEWER_GRID && z >= 0 && z < VIEWER_GRID {
        grid.set(x, y, z, mat as u8)
    }
}

pub fn paint(grid: VoxelGrid, x: i32, y: i32) {
    set_if(grid, x, y, 0, 1)
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("grid.wj", GRID);
    test.add_file("viewer.wj", VIEWER);
    test
}

#[test]
fn engine_i32_formal_into_owned_copy_set_must_not_star_deref_clone() {
    let test = fixture();
    let map = test
        .compile()
        .expect("engine i32 set_if multipass compile should succeed");
    let rs = map.get("viewer.rs").expect("viewer.rs");
    eprintln!("engine viewer.rs:\n{rs}");

    assert!(
        !rs.contains("*x.clone()") && !rs.contains("*y.clone()"),
        "owned i32 formal must not emit *binding.clone() (E0614). Got:\n{rs}"
    );
    assert!(
        !rs.contains("set(*x") && !rs.contains("set(*y") && !rs.contains(", *x") && !rs.contains(", *y"),
        "owned i32 formal must not be star-deref'd into Copy set formals (E0614). Got:\n{rs}"
    );
    assert!(
        rs.contains("grid.set(x") || rs.contains(".set(x,"),
        "expected bare x at set call site. Got:\n{rs}"
    );
}
