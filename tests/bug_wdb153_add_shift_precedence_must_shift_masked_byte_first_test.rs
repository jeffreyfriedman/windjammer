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

//! WDB-153: `a + (b as u32) << shift` must not emit `(a + b) << shift`.
//!
//! WindjammerDB CQ-C5: tip codegen of protobuf varint
//!   `value = value + ((b & 127) as u32) << shift`
//! emitted `(value + (b & 127) as u32) << shift` (Rust precedence),
//! breaking multi-byte varints. Product fixed with intermediate `piece`.
//!
//! Expected: shift applies to the 7-bit group before addition:
//!   `value = value + (((b & 127) as u32) << shift)`
//! or equivalent.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn accumulate(value: u32, b: u8, shift: u32) -> u32 {
    value + ((b & 127) as u32) << shift
}
"#;

#[test]
fn wdb153_shift_applies_to_masked_byte_before_add() {
    let mut test = MultiFileTest::new();
    test.add_file("main.wj", SRC);
    let map = test
        .compile()
        .expect("WDB-153 compile should succeed (codegen may still be wrong)");
    let rs = map.values().next().expect("generated rs");
    eprintln!("WDB-153 generated:\n{rs}");

    let bad = rs.contains("(value +") && rs.contains(") << shift");
    let _good = rs.contains("<< shift")
        && (rs.contains("value + ((") || rs.contains("value + (((") || rs.contains("+ (("));

    // Accept intermediate binding pattern as well.
    let piece_ok = rs.contains("<< shift") && !bad;

    if bad || !piece_ok {
        // If tip parenthesizes as (value + x) << shift → RED
        if rs.contains("(value +") && rs.contains(") <<") {
            panic!(
                "WDB-153 RED: `value + ((b & 127) as u32) << shift` must not emit `(value + …) << shift`. \
                 Product: otlp_protobuf_varint_decode (dogfood uses intermediate piece)."
            );
        }
    }

    // Stronger: must not have the bad grouping
    if rs.contains("(value +") && rs.contains(") << shift") {
        panic!("WDB-153 RED: shift grouped with addition accumulator");
    }

    // Must emit shift as RHS of add (Rust needs parens because WJ binds shift tighter).
    assert!(
        _good || (rs.contains("value +") && rs.contains("<< shift") && !bad),
        "WDB-153: expected `value + (… << shift)`, got:\n{rs}"
    );

    test.cargo_check()
        .expect("WDB-153: accumulate helper must cargo-check");
}
