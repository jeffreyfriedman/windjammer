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

//! borrowed `&Grid` formal must not receive owned `grid.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

// Force borrowed grid formal via read-only use (no field mutation).
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
fn module_file_must_not_pass_owned_clone_into_borrowed_or_ref_sample() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    // Accept either owned Grid formal + owned arg, or &Grid + &grid — not mixed.
    let formal_borrowed = rs.contains("grid: &Grid") || rs.contains("grid: &mut Grid");
    let call_owned_clone = rs.contains("grid.clone()");
    let call_ref_sample = rs.contains("&sample");
    let bad_mix = formal_borrowed && call_owned_clone;
    let bad_ref = call_ref_sample;
    assert!(
        !bad_mix && !bad_ref,
        "RED borrowed/owned polarity mismatch or &sample:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
