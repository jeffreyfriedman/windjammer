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

//! indexed Copy Vec3 must not emit `].clone().position.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Vertex {
    pub position: Vec3,
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
}

impl Mesh {
    pub fn collect_positions(self) -> Vec<Vec3> {
        let mut out = Vec::new()
        let mut i = 0
        while i < self.vertices.len() {
            out.push(self.vertices[i].position)
            i = i + 1
        }
        out
    }
}
"#;

#[test]
fn module_file_copy_vec3_field_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().position") || rs.contains(".position.clone()");
    assert!(
        !bad,
        "RED indexed Copy Vec3 double-cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
