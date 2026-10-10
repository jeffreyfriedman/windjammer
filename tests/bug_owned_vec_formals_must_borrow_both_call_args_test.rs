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

//! if both owned `Vec` formals are demoted to `&Vec`, both call args borrow.
//!
//! `half_edge.wj` declares `from_triangle_mesh(positions: Vec<Vec3>, indices: Vec<u32>)`.
//! `mesh_ops.rs` calls `from_triangle_mesh(positions, &indices)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn mixed_borrow(body: &str) -> bool {
    body.contains("from_triangle_mesh(positions, &indices)")
        || body.contains("from_triangle_mesh(positions,&indices)")
}

#[test]
fn owned_vec_formals_must_borrow_both_call_args() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod mesh
pub mod ops
"#,
    );
    test.add_file(
        "mesh.wj",
        r#"
pub struct Vec3 {
    pub x: f32,
}
pub struct Mesh {}
impl Mesh {
    pub fn from_triangle_mesh(positions: Vec<Vec3>, indices: Vec<u32>) -> Mesh {
        Mesh {}
    }
}
"#,
    );
    test.add_file(
        "ops.wj",
        r#"
use crate::mesh::Vec3
use crate::mesh::Mesh
pub fn build(positions: Vec<Vec3>, indices: Vec<u32>) -> Mesh {
    Mesh::from_triangle_mesh(positions, indices)
}
"#,
    );
    let map = test
        .compile()
        .expect("owned Vec formals fixture must transpile");
    let body = map
        .get("ops.rs")
        .unwrap_or_else(|| panic!("ops.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !mixed_borrow(body),
        "both Vec args must be passed the same way; got:\n{body}"
    );
}

