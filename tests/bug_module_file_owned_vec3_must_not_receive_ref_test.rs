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

//! owned Custom (`Vec3`) formal must not receive `&test_x`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Grid {
    pub ok: bool,
}

pub fn collides_aabb(grid: Grid, pos: Vec3, scale: i32) -> bool {
    grid.ok && scale > 0 && (pos.x + pos.y + pos.z) > 0.0
}

pub fn try_move(grid: Grid, test_x: Vec3, scale: i32) -> bool {
    collides_aabb(grid, test_x, scale)
}
"#;

#[test]
fn module_file_owned_vec3_must_not_receive_ref() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("pos: Vec3") || rs.contains("mut pos: Vec3");
    assert!(owned, "expected owned Vec3 formal:\n{rs}");
    let bad = rs.contains("&test_x") || rs.contains("collides_aabb(grid, &");
    assert!(
        !bad,
        "RED owned Vec3 received &test_x:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
