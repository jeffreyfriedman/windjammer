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

//! a Copy `f32` struct field must not `.clone()` when read into another struct.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Src {
    pub albedo: Vec3,
    pub roughness: f32,
}

pub struct Mat {
    pub color_r: f32,
    pub roughness: f32,
}

pub fn pack(m: Src) -> Mat {
    Mat { color_r: m.albedo.x, roughness: m.roughness }
}
"#;

#[test]
fn module_file_copy_f32_struct_field_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()");
    assert!(
        !bad,
        "RED Copy f32 field cloned:\n{rs}"
    );
    let _ = test.cargo_check();
}
