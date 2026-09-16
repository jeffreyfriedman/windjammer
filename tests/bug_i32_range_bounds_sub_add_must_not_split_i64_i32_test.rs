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

//! P3.318: `for x in (cx - r - 2)..(cx + r + 2)` with i32 locals must not emit
//! `cx as i64 - r - 2_i32` (component_viewer_controls / voxel ring loops).
//!
//! P3.313 covers `0..(zd + 1)`; this gate is the symmetric subtract/add bound chain.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod viewer
"#;

const VIEWER: &str = r#"
pub const GRID: i32 = 64

pub fn ring_scan() {
    let cx = GRID / 2
    let outer_r = 11
    for x in (cx - outer_r - 2)..(cx + outer_r + 2) {
        let _ = x
    }
}
"#;

fn bad_range_bound_split(rs: &str) -> bool {
    rs.contains("as i64 -")
        || rs.contains("as i64 +")
        || (rs.contains("..") && rs.contains("_i32") && rs.contains(" as i64"))
}

#[test]
fn i32_range_bounds_sub_add_must_not_split_i64_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("viewer.wj", VIEWER);
    let map = test.compile().expect("P3.318 compile");
    let rs = map.get("viewer.rs").expect("viewer.rs");
    if bad_range_bound_split(rs) {
        eprintln!("P3.318 RED:\n{rs}");
    }
    assert!(
        !bad_range_bound_split(rs),
        "P3.318: i32 range bounds must stay i32 width (no i64/i32 split):\n{rs}"
    );
    test.cargo_check().expect("P3.318 cargo-check");
}
