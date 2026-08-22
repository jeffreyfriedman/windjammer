//! `std::random.range` must codegen to `windjammer_runtime::random::int_range`.
//!
//! Ecosystem `wj-uuid`: `random.range(0, 256)` for v4 octets. Today emits
//! `random::range(...)` (E0425: not found in `random`).

#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn std_random_range_codegen_uses_int_range() {
    let source = r#"
use std::random

pub fn octet() -> int {
    random.range(0, 256)
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("random::int_range(0, 256)")
            || generated.contains("random::int_range(0_i64, 256_i64)"),
        "std::random.range must map to runtime int_range, got:\n{generated}"
    );
    assert!(
        !generated.contains("random::range("),
        "must not emit nonexistent random::range, got:\n{generated}"
    );
}
