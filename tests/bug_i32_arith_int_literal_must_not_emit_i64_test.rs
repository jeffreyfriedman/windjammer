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

//! P3.322: i32 locals ± untyped int literals in arithmetic/comparisons must not emit `_i64`
//! peers (`(outer_r - 1_i64) * …`) — component_viewer_controls dist_sq.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod viewer
"#;

const VIEWER: &str = r#"
pub const GRID: i32 = 64

pub fn ring_dist_check(dist_sq: i32) {
    let cx = GRID / 2
    let outer_r = 11
    let inner_r = 7
    if dist_sq <= (outer_r - 1) * (outer_r - 1) && dist_sq >= (inner_r + 1) * (inner_r + 1) {
        let _ = cx
    }
}
"#;

fn bad_i32_arith_i64_literal(rs: &str) -> bool {
    rs.contains("- 1_i64")
        || rs.contains("+ 1_i64")
        || rs.contains("(outer_r + 1_i64)")
        || rs.contains("(outer_r - 1_i64)")
        || rs.contains("(inner_r + 1_i64)")
}

#[test]
fn i32_arith_int_literal_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("viewer.wj", VIEWER);
    let map = test.compile().expect("P3.322 compile");
    let rs = map.get("viewer.rs").expect("viewer.rs");
    if bad_i32_arith_i64_literal(rs) {
        eprintln!("P3.322 RED:\n{rs}");
    }
    assert!(
        !bad_i32_arith_i64_literal(rs),
        "P3.322: i32 arith with int literals must stay i32 width:\n{rs}"
    );
    test.cargo_check().expect("P3.322 cargo-check");
}
