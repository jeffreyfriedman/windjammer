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

//! P3.356: Voxel coord / GPU dimension math must not emit `_i64` literal peers into i32 `set_if`
//! or u32 screen dimensions (`door_w / 2`, `hw + 1`, `(w + 3) / 4`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod grid
pub mod viewer
pub mod gpu_buf
"#;

const GRID: &str = r#"
pub struct VoxelGrid {
    pub data: Vec<i32>,
}

pub fn set_if(grid: VoxelGrid, x: i32, y: i32, z: i32, mat: i32) {
    let _ = grid
    let _ = x
    let _ = y
    let _ = z
    let _ = mat
}

pub fn place_blast_door(grid: VoxelGrid, x: i32, z: i32, floor_y: i32, along_x: bool) {
    let hw = 1
    for y in floor_y + 1..floor_y + 7 {
        if along_x {
            set_if(grid, x - (hw + 1), y, z, 0)
            set_if(grid, x + (hw + 1), y, z, 0)
        } else {
            set_if(grid, x, y, z - (hw + 1), 0)
            set_if(grid, x, y, z + (hw + 1), 0)
        }
    }
}
"#;

const VIEWER: &str = r#"
const VIEWER_GRID: int = 64

fn build_door_frame(grid: VoxelGrid) {
    let cx = VIEWER_GRID / 2
    let door_w = 8
    let frame_w = 2
    let wall_left = cx - door_w / 2 - frame_w
    let _ = wall_left
    set_if(grid, cx - door_w / 2, 1, cx, 0)
}
"#;

const GPU_BUF: &str = r#"
extern fn gpu_get_screen_width() -> u32

pub fn hiz_dims() -> u32 {
    let w = gpu_get_screen_width()
    let hiz_w = (w + 3) / 4
    hiz_w
}
"#;

fn bad_i64_coord_peers(rs: &str) -> bool {
    rs.contains("/ 2_i64")
        || rs.contains("+ 1_i64")
        || rs.contains("+ 3_i64")
        || rs.contains("/ 4_i64")
        || rs.contains("- (hw + 1_i64)")
        || rs.contains("+ (hw + 1_i64)")
}

#[test]
fn i32_coord_literal_peers_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("grid.wj", GRID);
    test.add_file("viewer.wj", VIEWER);
    test.add_file("gpu_buf.wj", GPU_BUF);
    let map = test.compile().expect("P3.353 compile");
    let combined = format!(
        "{}\n{}\n{}",
        map.get("grid.rs").expect("grid.rs"),
        map.get("viewer.rs").expect("viewer.rs"),
        map.get("gpu_buf.rs").expect("gpu_buf.rs")
    );
    if bad_i64_coord_peers(&combined) {
        eprintln!("P3.353 RED:\n{combined}");
    }
    assert!(
        !bad_i64_coord_peers(&combined),
        "P3.353: coord literal peers must not emit i64 suffixes:\n{combined}"
    );
    test.cargo_check().expect("P3.353 cargo-check");
}
