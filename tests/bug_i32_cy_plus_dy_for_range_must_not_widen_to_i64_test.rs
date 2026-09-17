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

//! P3.347: `let cy = 10` + `for dy in 0..2 { let y = cy + dy; set_if(..., y, ...) }`
//! must not emit `cy + dy as i64` (component_viewer_controls ring body).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod builder
pub mod viewer
"#;

const BUILDER: &str = r#"
pub struct VoxelGrid {}

pub fn set_if(grid: VoxelGrid, x: i32, y: i32, z: i32, mat: i32) {
    let _ = (grid, x, y, z, mat)
}
"#;

const VIEWER: &str = r#"
use crate::builder::set_if
use crate::builder::VoxelGrid

const GRID: i32 = 32

pub fn ring_layer(grid: VoxelGrid) {
    let cx = GRID / 2
    let cy = 10
    let cz = GRID / 2
    let outer_r = 11
    for dy in 0..2 {
        let y = cy + dy
        for x in cx - outer_r..cx + outer_r {
            set_if(grid, x, y, cz, 1)
        }
    }
}
"#;

fn bad_widen(rs: &str) -> bool {
    rs.contains("as i64")
        || rs.contains("+ dy as i64")
        || rs.contains("cy + dy as i64")
}

#[test]
fn i32_cy_plus_dy_for_range_must_not_widen_to_i64() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("builder.wj", BUILDER);
    test.add_file("viewer.wj", VIEWER);
    let map = test.compile().expect("P3.347 compile");
    let rs = map.get("viewer.rs").expect("viewer.rs");
    if bad_widen(rs) {
        eprintln!("P3.347 RED:\n{rs}");
    }
    assert!(
        !bad_widen(rs),
        "P3.347: cy + dy for-range must stay i32, not widen to i64:\n{rs}"
    );
    test.cargo_check().expect("P3.347 cargo-check");
}
