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

//! Regression: module file copy f32 field must not emit clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Cam {
    pub pivot: Vec3,
}

pub fn sample(cam: Cam, dx: f32) -> Vec3 {
    Vec3 {
        x: cam.pivot.x + dx,
        y: cam.pivot.y,
        z: cam.pivot.z,
    }
}
"#;

#[test]
fn module_file_copy_f32_field_must_not_emit_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()");
    assert!(
        !bad,
        "RED: Copy f32 field access emitted .clone():\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
