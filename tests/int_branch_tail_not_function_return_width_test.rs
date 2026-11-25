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

//! The tail of an `if`/`else` that is a value inside an expression (`sum += if c { v } else { 256 - v }`)
//! is not the function's implicit return: a `-> Vec<u8>` function must not make that tail a `u8`
//! (WJ0027 "must be U8 (implicit return) but was I64").

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn if_value_in_compound_assign_is_not_the_vec_u8_return() {
    let mut t = MultiFileTest::new();
    t.add_file("mod.wj", "pub mod rows\n");
    t.add_file(
        "rows.wj",
        r#"
pub fn fold(v: int) -> Vec<u8> {
    let mut raw: Vec<u8> = []
    let mut sum = 0
    sum += if v < 128 { v } else { 256 - v }
    raw.push(sum as u8)
    raw
}
"#,
    );
    let out = t.compile();
    assert!(out.is_ok(), "int `if` value under a `-> Vec<u8>` function must compile: {:?}", out.err());
}

#[test]
fn function_tail_if_still_gets_the_return_width() {
    let mut t = MultiFileTest::new();
    t.add_file("mod.wj", "pub mod rows\n");
    t.add_file(
        "rows.wj",
        r#"
pub fn pick(v: int) -> u8 {
    if v < 128 { 1 } else { 2 }
}
"#,
    );
    let out = t.compile().expect("compile");
    let rs = out.get("rows.rs").expect("rows.rs");
    assert!(rs.contains("-> u8"), "{}", rs);
}
