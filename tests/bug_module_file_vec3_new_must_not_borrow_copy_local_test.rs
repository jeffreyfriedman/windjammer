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

//! WDB-388: Copy locals into `Vec3::new(x: f32, …)` must not emit `Vec3::new(&x, …)`.
//!
//! P3.442 isolate + navmesh tip exist; product `gen/` still over-borrows the first
//! f32 ident in 30+ files (cameras, mesh_generator, voxel_mesh, terrain, …).
//! WJ source is `Vec3::new(x, y, z)`. Twin of P3.442 with full-product scan.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
    }
}

pub fn point(a: f32, b: f32, c: f32) -> Vec3 {
    Vec3::new(a, b, c)
}
"#;

#[test]
fn module_file_vec3_new_must_not_borrow_copy_local() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("Vec3::new(&a"),
        "Copy local borrowed into Vec3::new:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
