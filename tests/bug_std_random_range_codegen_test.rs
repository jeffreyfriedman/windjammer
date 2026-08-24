//! `std::random.range` must codegen to `windjammer_runtime::random::int_range`.
//!
//! WJ exposes `range`; runtime Rust fn is `int_range`. Alias wiring must survive
//! `cargo check` (not just transpile).

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
    let generated = test_utils::assert_stdlib_runtime_links(source, &["random::int_range"]);
    assert!(
        !generated.contains("random::range("),
        "must not emit nonexistent random::range, got:\n{generated}"
    );
}
