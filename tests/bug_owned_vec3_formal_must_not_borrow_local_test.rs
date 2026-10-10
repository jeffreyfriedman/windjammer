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

//! P3.732: a local `Vec3` passed to an owned `Vec3` formal must not be borrowed.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn owned_vec3_formal_must_not_borrow_local() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
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
pub struct Grid {}
pub fn collides(grid: Grid, pos: Vec3) -> bool {
    pos.x > 0.0
}
pub fn move_it(grid: Grid) {
    let test_x = Vec3::new(1.0, 2.0, 3.0)
    collides(grid, test_x)
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.732: owned Vec3 formal fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !body.contains("&test_x"),
        "owned Vec3 formal must not borrow the local; got:\n{body}"
    );
}
