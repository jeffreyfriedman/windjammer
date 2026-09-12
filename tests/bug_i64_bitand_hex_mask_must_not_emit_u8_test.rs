//! Hex / shift masks in `i64` byte packing must not pick up `u8` from `Vec::push`.
//!
//! Ecosystem `wj-uuid` (v1/v7):
//! ```
//! out.push(((unix_millis >> 40) & 0xff) as u8)
//! ```
//! Tip CLI codegen emits `unix_millis >> 40_u8 & 255_u8` → E0277 `i64 & u8`.
//! Bare `(value & 0xff) as u8` correctly emits `255_i64`; the bug is context
//! pollution from `Vec<u8>::push` (and/or nested shift+mask).
//!
//! Expected: shift amounts and masks stay `i64`-compatible (`40_i64`, `255_i64`).

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
fn i64_bitand_hex_mask_must_not_emit_u8_literal() {
    let source = r#"
pub fn pack_millis(unix_millis: i64) -> Vec<u8> {
    let mut out = Vec::new()
    out.push(((unix_millis >> 40) & 0xff) as u8)
    out.push(((unix_millis >> 8) & 0xff) as u8)
    out.push((unix_millis & 0xff) as u8)
    out
}
"#;
    let (generated, ok) = test_utils::compile_single_check(source);
    assert!(
        ok,
        "i64 shift/mask into Vec<u8> push must type-check, got:\n{generated}"
    );
    assert!(
        !generated.contains("255_u8") && !generated.contains("_u8 &"),
        "0xff / shift literals must stay i64-compatible inside Vec<u8> push, got:\n{generated}"
    );
}
