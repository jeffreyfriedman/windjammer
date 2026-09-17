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

//! P3.343: Nested `for dy in 0..3` / `for dx in 0..4` with `dy == 1`, `base + 2 + dy`,
//! and `(dx + dz) % 2 == 0` must not emit `_i64` peers (component_viewer_controls vents/checker).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod viewer
"#;

const VIEWER: &str = r#"
pub fn vent_sample(base_y: i32) -> bool {
    for dy in 0..3 {
        for dx in 0..2 {
            if dy == 1 {
                let y = base_y + 2 + dy
                if y > 0 {
                    return true
                }
            }
        }
    }
    false
}

pub fn checker_sample() -> bool {
    for dx in 0..4 {
        for dz in 0..4 {
            if (dx + dz) % 2 == 0 {
                return true
            }
        }
    }
    false
}
"#;

fn bad_i32_nested_range_i64_literal(rs: &str) -> bool {
    rs.contains("== 1_i64")
        || rs.contains("+ 2_i64")
        || rs.contains("% 2_i64")
        || rs.contains("== 0_i64")
}

#[test]
fn i32_nested_range_eq_mod_literals_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("viewer.wj", VIEWER);
    let map = test.compile().expect("P3.343 compile");
    let rs = map.get("viewer.rs").expect("viewer.rs");
    if bad_i32_nested_range_i64_literal(rs) {
        eprintln!("P3.343 RED:\n{rs}");
    }
    assert!(
        !bad_i32_nested_range_i64_literal(rs),
        "P3.343: i32 nested range loops must not emit _i64 int literals:\n{rs}"
    );
    test.cargo_check().expect("P3.343 cargo-check");
}
