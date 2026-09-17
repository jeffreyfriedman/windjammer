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

//! P3.353: i32 coord math must not emit `_i64` literal peers or `as i64` in set_if / buffer sizing.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod builder
pub mod viewer
pub mod gpu
pub mod station
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

const VIEWER_GRID: int = 64

fn build_door_frame(grid: VoxelGrid) {
    let cx = VIEWER_GRID / 2
    let cz = VIEWER_GRID / 2
    let door_w = 8
    let wall_left = cx - door_w / 2 - 2
    for x in wall_left..(cx + door_w / 2) {
        set_if(grid, x, 1, cz, 1)
    }
}
"#;

const GPU: &str = r#"
pub fn hiz_dims(w: i32, h: i32) -> i32 {
    let hiz_w = (w + 3) / 4
    let hiz_h = (h + 3) / 4
    hiz_w * hiz_h
}
"#;

const STATION: &str = r#"
use crate::builder::set_if
use crate::builder::VoxelGrid

pub fn armor_rim(grid: VoxelGrid, x: i32, y: i32, z: i32, hw: i32) {
    set_if(grid, x - (hw + 1), y, z, 1)
    set_if(grid, x + hw + 1, y, z, 1)
}
"#;

fn bad_i64_coord_peers(rs: &str) -> bool {
    rs.contains("/ 2_i64")
        || rs.contains("+ 3_i64")
        || rs.contains("/ 4_i64")
        || rs.contains("+ 1_i64")
        || rs.contains("as i64")
}

#[test]
fn i32_coord_literal_peers_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("builder.wj", BUILDER);
    test.add_file("viewer.wj", VIEWER);
    test.add_file("gpu.wj", GPU);
    test.add_file("station.wj", STATION);
    let map = test.compile().expect("P3.353 compile");
    let combined = format!(
        "{}\n{}\n{}\n{}",
        map.get("viewer.rs").expect("viewer.rs"),
        map.get("gpu.rs").expect("gpu.rs"),
        map.get("station.rs").expect("station.rs"),
        ""
    );
    if bad_i64_coord_peers(&combined) {
        eprintln!("P3.353 RED:\n{combined}");
    }
    assert!(
        !bad_i64_coord_peers(&combined),
        "P3.353: i32 coord/div literals must not use i64 peers:\n{combined}"
    );
    test.cargo_check().expect("P3.353 cargo-check");
}
