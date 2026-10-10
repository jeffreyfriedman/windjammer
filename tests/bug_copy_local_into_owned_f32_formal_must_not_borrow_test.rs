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

//! local Copy scalars into owned Copy formals must not emit `&x`.
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

pub struct Triangle {
    pub id: u32,
}

impl Triangle {
    pub fn new(id: u32, a: Vec3, b: Vec3, c: Vec3) -> Triangle {
        Triangle { id: id }
    }
}

pub fn point(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

pub fn tri(id: u32, v0: Vec3, v1: Vec3, v2: Vec3) -> Triangle {
    Triangle::new(id, v0, v1, v2)
}
"#;

#[test]
fn copy_local_into_owned_f32_formal_must_not_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("Vec3::new(&x") && !rs.contains("Vec3::new(&x,"),
        "RED: local f32 into Vec3::new must not borrow:\n{rs}"
    );
    assert!(
        !rs.contains("Triangle::new(&id"),
        "RED: local u32 into Triangle::new must not borrow:\n{rs}"
    );
    assert!(
        rs.contains("Vec3::new(x") || rs.contains("Vec3::new(x,"),
        "expected Vec3::new(x, y, z):\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

/// `Vec3` in math/vec3.wj, call in ai/navmesh.wj.
#[test]
fn cross_module_copy_local_into_vec3_new_must_not_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", "pub mod math\npub mod nav\n");
    test.add_file("math/mod.wj", "pub mod vec3\n");
    test.add_file(
        "math/vec3.wj",
        r#"
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
"#,
    );
    test.add_file(
        "nav.wj",
        r#"
use super::math::vec3::Vec3

pub fn point(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}
"#,
    );
    let map = test.compile().expect("compile");
    let rs = map
        .get("nav.rs")
        .or_else(|| map.get("nav/mod.rs"))
        .expect("nav.rs");
    eprintln!("cross-module nav.rs:\n{rs}");
    assert!(
        !rs.contains("Vec3::new(&x"),
        "RED: cross-module local f32 into Vec3::new must not borrow:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
