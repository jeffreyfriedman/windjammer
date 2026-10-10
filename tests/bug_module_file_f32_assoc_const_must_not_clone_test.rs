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
//!   `Vec3::new(f32::MAX.clone(), f32::MAX.clone(), f32::MAX.clone())`
//! WJ source is `Vec3::new(f32::MAX, f32::MAX, f32::MAX)`. Prefer bare `f32::MAX`.

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

pub fn bounds_min() -> Vec3 {
    Vec3::new(f32::MAX, f32::MAX, f32::MAX)
}

pub fn bounds_max() -> Vec3 {
    Vec3::new(f32::MIN, f32::MIN, f32::MIN)
}
"#;

#[test]
fn module_file_f32_assoc_const_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("f32::MAX.clone()") || rs.contains("f32::MIN.clone()");
    assert!(
        !bad,
        "RED: f32 associated const cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

