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

//! Engine tip blocker (windjammer-game-core `component_viewer_controls.wj`):
//!
//!   `for x in cx - 16_i64..cx + 16_i64` with `cx: i32` → E0277 / E0308
//!   (`i32 ± i64` / mismatched range bounds)
//!
//! Integer literals in an `i32` arithmetic / range context must emit as `i32`
//! (bare `16` or `16_i32`), never default WJ `16_i64`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod pedestal
"#;

const PEDESTAL: &str = r#"
const VIEWER_GRID: i32 = 64

pub fn build_pedestal() -> i32 {
    // Product shape: cx from const / 2, then range + edge compares with literals.
    let cx = VIEWER_GRID / 2
    let mut count = 0
    for x in (cx - 16)..(cx + 16) {
        let edge = x == cx - 16 || x == cx + 15
        if edge {
            count = count + 1
        }
    }
    count
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("pedestal.wj", PEDESTAL);
    test
}

#[test]
fn engine_i32_range_literal_must_not_emit_i64_suffix() {
    let test = fixture();
    let map = test
        .compile()
        .expect("engine i32 range multipass compile should succeed");
    let rs = map.get("pedestal.rs").expect("pedestal.rs");
    eprintln!("engine pedestal.rs:\n{rs}");

    assert!(
        !rs.contains("16_i64") && !rs.contains("15_i64"),
        "i32 range/peer arithmetic must not emit _i64 literals. Product: \
         for x in cx - 16_i64..cx + 16_i64. Got:\n{rs}"
    );
    assert!(
        rs.contains("cx - 16") || rs.contains("cx - 16_i32") || rs.contains("(cx - 16)"),
        "expected i32-width range start. Got:\n{rs}"
    );
}
