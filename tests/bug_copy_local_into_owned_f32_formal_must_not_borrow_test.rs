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

//! P3.442: local Copy scalars into owned Copy formals must not emit `&x`.
//!
//! Product tip `gen/ai/navmesh.rs`:
//!   `Vec3::new(&x, y, z)` where `new(x: f32, y: f32, z: f32)`
//!   `Triangle::new(&id, v0, v1, v2)` where `id: u32`
//! Source is `Vec3::new(x, y, z)` — first identifier is over-borrowed.
//! Literals / field args already have gates; this is the local-ident hole.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

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
    let map = test.compile().expect("P3.442 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.442 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("Vec3::new(&x") && !rs.contains("Vec3::new(&x,"),
        "P3.442 RED: local f32 into Vec3::new must not borrow:\n{rs}"
    );
    assert!(
        !rs.contains("Triangle::new(&id"),
        "P3.442 RED: local u32 into Triangle::new must not borrow:\n{rs}"
    );
    assert!(
        rs.contains("Vec3::new(x") || rs.contains("Vec3::new(x,"),
        "P3.442 expected Vec3::new(x, y, z):\n{rs}"
    );
    test.cargo_check().expect("P3.442 cargo-check");
}

/// Product shape: `Vec3` in math/vec3.wj, call in ai/navmesh.wj.
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
    let map = test.compile().expect("P3.442b compile");
    let rs = map
        .get("nav.rs")
        .or_else(|| map.get("nav/mod.rs"))
        .expect("nav.rs");
    eprintln!("P3.442b cross-module nav.rs:\n{rs}");
    assert!(
        !rs.contains("Vec3::new(&x"),
        "P3.442b RED: cross-module local f32 into Vec3::new must not borrow:\n{rs}"
    );
    test.cargo_check().expect("P3.442b cargo-check");
}

#[test]
fn tip_out_navmesh_vec3_new_must_not_borrow_local_f32() {
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core/gen/ai/navmesh.rs");
    assert!(game.exists(), "missing tip navmesh");
    let text = std::fs::read_to_string(&game).expect("product");
    assert!(
        !text.contains("Vec3::new(&x") && !text.contains("Triangle::new(&id"),
        "P3.442 RED: tip navmesh still over-borrows Copy locals into owned formals"
    );
}
