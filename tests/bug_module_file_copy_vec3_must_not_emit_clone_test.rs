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

//! Copy `Vec3` must not emit `.clone()` into owned formals.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub fn vertex(pos: Vec3, normal: Vec3) -> Vec3 {
    Vec3 {
        x: pos.x + normal.x,
        y: pos.y + normal.y,
        z: pos.z + normal.z,
    }
}

pub fn face(n: Vec3, p: Vec3) -> Vec3 {
    vertex(p, n)
}
"#;

#[test]
fn module_file_copy_vec3_must_not_emit_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()");
    assert!(
        !bad,
        "RED Copy Vec3 emitted .clone():\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
