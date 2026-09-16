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

//! P3.312: `for zi in 0..(zd + 1)` with `zd: i32` must not emit `zd as i64 + 1_i32`.
//!
//! Product: `mesh_primitives.wj` → E0277 / E0308 on range end expression.
//! (P3.311 is the dotenv vec-index `i += 1 as i32` gate — do not collide.)

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod grid
"#;

const GRID: &str = r#"
pub fn walk(z_divisions: i32) {
    let mut zd = z_divisions
    if zd < 1 {
        zd = 1
    }
    for zi in 0..(zd + 1) {
        let _ = zi
    }
}
"#;

fn bad_range_add_split(rs: &str) -> bool {
    rs.contains("as i64 + 1_i32")
        || rs.contains("as i64 + 1_i32")
        || (rs.contains("..") && rs.contains(" as i64 + ") && rs.contains("_i32"))
}

#[test]
fn i32_range_end_add_must_not_split_i64_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("grid.wj", GRID);
    let map = test.compile().expect("P3.312 compile");
    let rs = map.get("grid.rs").expect("grid.rs");
    if bad_range_add_split(rs) {
        eprintln!("P3.312 RED:\n{rs}");
    }
    assert!(
        !bad_range_add_split(rs),
        "RED P3.312: i32 range end add must stay i32 width, not i64 + i32 split:\n{rs}"
    );
    test.cargo_check().expect("P3.312 cargo-check");
}
