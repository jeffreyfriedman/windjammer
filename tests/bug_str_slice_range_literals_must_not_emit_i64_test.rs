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

//! P3.354 — `string`/`&str` slice ranges `s[0..1]` must use `usize` bounds, not `_i64`.
//!
//! Product residual (finance-screens tip regen `theme.rs`):
//!   `hex_digit_value(&(pair[0_i64..1_i64]))` → E0277 slice index expects usize.

#[path = "common/test_utils.rs"]
mod test_utils;

const SOURCE: &str = r#"
fn hex_digit_value(s: string) -> int {
    0
}

pub fn parse_hex_byte(pair: string) -> int {
    if pair.len() != 2 {
        return -1
    }
    let hi = hex_digit_value(pair[0..1])
    let lo = hex_digit_value(pair[1..2])
    if hi < 0 || lo < 0 {
        return -1
    }
    hi * 16 + lo
}
"#;

fn bad_str_slice_range_bounds(rs: &str) -> bool {
    rs.contains("0_i64..")
        || rs.contains("1_i64..")
        || rs.contains("2_i64")
        || rs.contains("[0_i64..")
        || rs.contains("[1_i64..")
        || rs.contains("..1_i64]")
        || rs.contains("..2_i64]")
}

#[test]
fn str_slice_range_literals_must_not_emit_i64() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    if bad_str_slice_range_bounds(&rs) {
        eprintln!("RED P3.354 str slice range _i64:\n{rs}");
    }
    assert!(
        !bad_str_slice_range_bounds(&rs),
        "P3.354: string slice range bounds must be usize (0_usize/0), not _i64. Got:\n{rs}"
    );
    assert!(
        ok,
        "P3.354: parse_hex_byte slice ranges must cargo-check. Generated:\n{rs}"
    );
}
