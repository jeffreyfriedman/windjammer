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

//! P3.372 Breach dogfood: int cast + auto-clone at call site must not emit `x as i32.clone()`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn place(grid: VoxelGrid, pi: i32) {
    let zs = [18, 20, 35, 38]
    let pz = zs[(pi as usize)]
    round_pillar(grid, 37, 1, 2, pz)
}

fn round_pillar(grid: VoxelGrid, lx: i32, ly_start: i32, ly_end: i32, lz: i32) {
    let _ = lx + ly_start + ly_end + lz
}
"#;

#[test]
fn i32_cast_before_clone_call_arg_must_parenthesize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.372 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    assert!(
        !rs.contains(" as i32.clone()"),
        "P3.372: cast must bind before .clone():\n{rs}"
    );
    test.cargo_check().expect("P3.372 cargo-check");
}
