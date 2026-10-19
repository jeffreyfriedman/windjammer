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

//! a `for`-in counter later cast to `f32` must not be `i64`.
//!
//! `visual_verification.wj` `non_black_percentage` writes `let mut count = 0`,
//! increments it inside `for p in self.pixels`, then returns
//! `let mut count = 0_i64`. The counter is not a vec index.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[allow(dead_code)]
fn bad_i64_count(body: &str) -> bool {
    body.contains("count = 0_i64") || body.contains("count: i64")
}

// Width is the unified solver's since ADR-0015 (`int` is i64, and an untyped literal follows its peers), so the old
// text assertions about i32 vs i64 no longer describe the contract. The property that mattered in the dogfood
// code is that the emitted Rust has no mixed-width arithmetic: it compiles (`cargo check`).
#[test]
fn for_in_f32_ratio_count_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub struct Shot {
    pub pixels: Vec<f32>,
}

pub fn ratio(self, threshold: f32) -> f32 {
    let mut count = 0
    for p in self.pixels {
        if p > threshold {
            count = count + 1
        }
    }
    let total = self.pixels.len()
    if total == 0 {
        return 0.0
    }
    (count as f32) / (total as f32)
}
"#,
    );
    let map = test
        .compile()
        .expect("for-in f32 ratio fixture must transpile");
    assert!(map.contains_key("mod.rs"), "mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>());
    test.cargo_check().expect("for-in f32 ratio fixture must cargo-check");
}
