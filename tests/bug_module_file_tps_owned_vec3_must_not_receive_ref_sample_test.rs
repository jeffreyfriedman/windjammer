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

//! Regression: module file tps owned vec3 must not receive ref sample.

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

pub fn collides_point(grid: Grid, pos: Vec3, scale: i32) -> bool {
    grid.ok && scale > 0 && (pos.x + pos.y + pos.z) > 0.0
}

pub fn sample_arm(grid: Grid, sample: Vec3, scale: i32) -> bool {
    collides_point(grid, sample, scale)
}
"#;

#[test]
fn module_file_tps_owned_vec3_must_not_receive_ref_sample() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("pos: Vec3") || rs.contains("mut pos: Vec3");
    assert!(owned, "expected owned Vec3 formal:\n{rs}");
    let bad = rs.contains("&sample") || rs.contains("collides_point(grid, &");
    assert!(
        !bad,
        "RED: owned Vec3 received &sample:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
