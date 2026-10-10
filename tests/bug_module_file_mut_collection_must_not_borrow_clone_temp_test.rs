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

//!
//!   `voxel/meshing.rs`: `greedy_mesh_axis(&grid, &mut quads.clone(), …)`
//!   `scene/component_viewer_controls.rs`: `build_pedestal(&mut grid.clone())`
//!   `assets/vox_loader_test.rs`: `push_chunk_id(&mut d.clone(), …)` (42 hits)
//! → E0716 / discarded mutation.
//! Prefer `&mut quads` / `&mut grid` / `&mut d`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn greedy_mesh_axis_on(quads: Vec<i32>, axis: i32) {
    quads.push(axis)
}

// must emit `&mut quads`, never `&mut quads.clone()`.
pub fn mesh_all() -> Vec<i32> {
    let mut quads: Vec<i32> = Vec::new()
    greedy_mesh_axis_on(quads, 0)
    greedy_mesh_axis_on(quads, 1)
    quads
}
"#;

#[test]
fn module_file_mut_collection_must_not_borrow_clone_temp() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("&mut quads.clone()")
        || rs.contains("&mut grid.clone()");
    assert!(
        !bad,
        "RED: mut collection received &mut <temp>.clone():\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

