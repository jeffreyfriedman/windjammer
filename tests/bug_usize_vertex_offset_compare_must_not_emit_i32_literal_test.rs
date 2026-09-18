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

//! P3.373: `usize` vertex offsets (`v2 = i2 * 3`) in i32-param / f32-return files must
//! peer-drive small literals in bounds checks (`v2 + 2 >= mesh.vertices.len()`), not `2_i32`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod mesh
pub mod lod
"#;

const MESH: &str = r#"
pub struct Mesh3D {
    pub indices: Vec<i32>,
    pub vertices: Vec<f32>,
    pub normals: Vec<f32>,
    pub uvs: Vec<f32>,
}

impl Mesh3D {
    pub fn new() -> Mesh3D {
        Mesh3D {
            indices: Vec::new(),
            vertices: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
        }
    }
}
"#;

const LOD: &str = r#"
use mesh::Mesh3D

fn triangle_area_for_index(mesh: Mesh3D, tri_order: i32) -> f32 {
    let base = (tri_order * 3) as usize
    if base + 2 >= mesh.indices.len() {
        return 0.0
    }
    let i0 = mesh.indices[base] as usize
    let i1 = mesh.indices[base + 1] as usize
    let i2 = mesh.indices[base + 2] as usize
    let v0 = i0 * 3
    let v1 = i1 * 3
    let v2 = i2 * 3
    if v2 + 2 >= mesh.vertices.len() {
        return 0.0
    }
    let ax = mesh.vertices[v1] - mesh.vertices[v0]
    let ay = mesh.vertices[v1 + 1] - mesh.vertices[v0 + 1]
    ax + ay
}
"#;

fn bad_usize_bounds_i32_literal(rs: &str) -> bool {
    rs.contains("v2 + 2_i32") || rs.contains("vb + 2_i32") || rs.contains("+ 2_i32 >=")
}

#[test]
fn usize_vertex_offset_compare_must_not_emit_i32_literal() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("mesh.wj", MESH);
    test.add_file("lod.wj", LOD);
    let map = test.compile().expect("P3.373 compile");
    let rs = map.get("lod.rs").expect("lod.rs");
    assert!(
        !bad_usize_bounds_i32_literal(rs),
        "P3.373: usize bounds checks must not emit i32 literal peers:\n{rs}"
    );
    test.cargo_check().expect("P3.373 cargo-check");
}
