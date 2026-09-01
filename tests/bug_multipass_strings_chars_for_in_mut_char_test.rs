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

//! FAILING REPRO — multipass `for ch in strings.chars(text)` must not emit `&mut char`
//! at `char_to_digit` call sites (E0277 / E0596).
//!
//! Ecosystem `wj-notes-api` / `wj-fetch` on wj 0.50.0 (2026-08-31) emits
//! `match char_to_digit(&mut ch)` and `if ch == '0'` with `ch: &mut char`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const PARSE_MODULE: &str = include_str!("fixtures/library_multipass/parse_positive_int_chars.wj");

#[test]
fn multipass_strings_chars_for_in_must_not_emit_mut_char() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod parse
"#,
    );
    test.add_file("parse.wj", PARSE_MODULE);

    let map = test.compile().expect("parse fixture should compile");
    let parse_rs = map.get("parse.rs").expect("parse.rs");

    assert!(
        !parse_rs.contains("char_to_digit(&mut ch)"),
        "RED: char loop variable must be `char`, not `&mut char`; emitted:\n{parse_rs}"
    );
    assert!(
        !parse_rs.contains("for mut ch in strings::chars"),
        "RED: for-in over strings.chars must not require `mut ch`; emitted:\n{parse_rs}"
    );
    test.cargo_check()
        .expect("multipass strings.chars parse fixture must cargo-check");
}
