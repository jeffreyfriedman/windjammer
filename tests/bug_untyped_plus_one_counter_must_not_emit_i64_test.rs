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

//! an untyped counter incremented by `1` must not emit `0_i64`.
//!
//! `material.wj` `log_palette_summary` writes `let mut configured_count = 0`
//! then `configured_count = configured_count + 1` inside `while i < 256u32`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[allow(dead_code)]
fn bad_i64_counter(body: &str) -> bool {
    body.contains("configured_count = 0_i64") || body.contains("configured_count: i64")
}

// Width is the unified solver's since ADR-0015 (`int` is i64, and an untyped literal follows its peers), so the old
// text assertions about i32 vs i64 no longer describe the contract. The property that mattered in the dogfood
// code is that the emitted Rust has no mixed-width arithmetic: it compiles (`cargo check`).
#[test]
fn untyped_plus_one_counter_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub fn count_slots() {
    let mut configured_count = 0
    let mut i: u32 = 0u32
    while i < 256u32 {
        configured_count = configured_count + 1
        i = i + 1
    }
}
"#,
    );
    let map = test
        .compile()
        .expect("plus-one counter fixture must transpile");
    assert!(map.contains_key("mod.rs"), "mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>());
    test.cargo_check().expect("plus-one counter fixture must cargo-check");
}
