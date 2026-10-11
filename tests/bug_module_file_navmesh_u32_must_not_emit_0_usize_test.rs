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

//! a game project navmesh `find_triangle` — `u32 = 0_usize` must not emit.
//!
//! Product tip gen (`a downstream game project/gen/ai/navmesh.rs`):
//!   `let mut i: u32 = 0_usize;` inside `find_triangle(...) -> Option<u32>`
//! with `while i < self.triangles.len()`. Twin of .

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Triangle {
    pub id: u32,
    pub walkable: bool,
}

pub struct Navmesh {
    pub triangles: Vec<Triangle>,
}

pub fn find_triangle(mesh: Navmesh, _x: f32) -> Option<u32> {
    let mut i = 0
    while i < mesh.triangles.len() {
        let tri = mesh.triangles[i]
        if tri.walkable {
            return Some(tri.id)
        }
        i = i + 1
    }
    None
}
"#;

#[test]
fn module_file_core_lib_navmesh_u32_must_not_emit_0_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("u32 = 0_usize") || rs.contains(": u32 = 0_usize");
    assert!(
        !bad,
        "MultiFile emitted u32 = 0_usize (must be 0_u32):\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
