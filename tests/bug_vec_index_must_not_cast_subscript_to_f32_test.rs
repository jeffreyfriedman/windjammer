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

//! P3.329: Vec subscript indices must cast to `usize`, never `f32` (lod_generator vb+1).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod lod
"#;

const LOD: &str = r#"
pub struct Mesh {
    pub vertices: Vec<f32>,
}

pub fn pick_vertex(mesh: Mesh, vb: int) -> f32 {
    mesh.vertices[vb + 1]
}

pub fn tri_sum(mesh: Mesh, vb: int) -> f32 {
    mesh.vertices[vb] + mesh.vertices[vb + 1] + mesh.vertices[vb + 2]
}
"#;

fn bad_index_f32_cast(rs: &str) -> bool {
    rs.contains(" as f32]")
        || rs.contains("+ 1) as f32")
        || rs.contains("+ 2) as f32")
}

#[test]
fn vec_index_must_not_cast_subscript_to_f32() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("lod.wj", LOD);
    let map = test.compile().expect("P3.329 compile");
    let rs = map.get("lod.rs").expect("lod.rs");
    if bad_index_f32_cast(rs) {
        eprintln!("P3.329 RED:\n{rs}");
    }
    assert!(
        !bad_index_f32_cast(rs),
        "P3.329: Vec index expressions must use usize casts, not f32:\n{rs}"
    );
    test.cargo_check().expect("P3.329 cargo-check");
}
