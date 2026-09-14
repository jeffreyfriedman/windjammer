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
//!   `use crate::scene::component_viewer_state::VIEWER_GRID`
//!   `let cx = VIEWER_GRID / 2`
//!   `for x in (cx - 16)..(cx + 16)` → was `16_i64` (E0277 / E0308)
//!
//! Cross-module imported `i32` consts must peer-drive range literals like
//! same-module consts (`16_i32`, not default WJ `16_i64`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod state
pub mod pedestal
"#;

const STATE: &str = r#"
pub const VIEWER_GRID: i32 = 64
"#;

const PEDESTAL: &str = r#"
use crate::state::VIEWER_GRID

pub fn build_pedestal() -> i32 {
    // Product shape: imported i32 const / 2, parenthesized range + edge compares.
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

// Product residual: `let cy = 10` in a non-int-returning builder must not force
// `cy + 16_i64` (component_viewer_controls ring / wall loops).
pub fn build_ring_band() -> i32 {
    let cy = 10
    let mut count = 0
    for y in (cy + 4)..(cy + 16) {
        count = count + 1
        let _ = y
    }
    count
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("state.wj", STATE);
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
    eprintln!("engine pedestal.rs (cross-module VIEWER_GRID):\n{rs}");

    assert!(
        !rs.contains("16_i64") && !rs.contains("15_i64"),
        "imported i32 const range/peer arithmetic must not emit _i64. Product: \
         component_viewer_controls for x in cx - 16_i64..cx + 16_i64. Got:\n{rs}"
    );
    assert!(
        rs.contains("cx - 16") || rs.contains("cx - 16_i32") || rs.contains("(cx - 16)"),
        "expected i32-width range start. Got:\n{rs}"
    );
}
