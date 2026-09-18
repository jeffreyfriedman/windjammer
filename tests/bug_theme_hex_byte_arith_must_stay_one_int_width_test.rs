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

//! P3.371: finance-screens theme hex — `hi * 16 + lo` after i64 digit helpers must not
//! mix `i64 + (lo as i32)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod theme
"#;

const THEME: &str = r#"
fn hex_digit_value(c: string) -> int {
    0
}

pub fn parse_hex_byte(pair: string) -> int {
    if pair.len() != 2 {
        return -1
    }
    let hi = hex_digit_value(pair[0..1])
    let lo = hex_digit_value(pair[1..2])
    if hi < 0 {
        return -1
    }
    if lo < 0 {
        return -1
    }
    hi * 16 + lo
}
"#;

fn bad_mix(rs: &str) -> bool {
    rs.contains("as i32") && (rs.contains("+ lo") || rs.contains("* 16"))
        || rs.contains("16_i64 + lo as i32")
        || rs.contains("+ lo as i32")
}

#[test]
fn theme_hex_byte_arith_must_stay_one_int_width() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("theme.wj", THEME);
    let map = test.compile().expect("P3.371 compile");
    let rs = map.get("theme.rs").expect("theme.rs");
    if bad_mix(rs) {
        eprintln!("P3.371 RED:\n{rs}");
    }
    assert!(
        !bad_mix(rs),
        "P3.371: hex byte arith must keep one int width (no i64+i32):\n{rs}"
    );
    test.cargo_check().expect("P3.371 cargo-check");
}
