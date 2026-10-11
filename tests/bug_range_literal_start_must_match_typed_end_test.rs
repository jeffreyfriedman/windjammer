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

//! `for i in 0..n` must give the literal start the SAME width as the typed end bound.
//! Regression: `n: int` produced `0_i32..n` (i64), failing rustc with a type mismatch.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn check_width(wj_ty: &str, extra_body: &str) {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        &format!(
            r#"
pub fn total(n: {wj_ty}, v: Vec<int>) -> int {{
    let mut c: int = 0
    for i in 0..n {{
        c = c + v[i]{extra_body}
    }}
    c
}}
"#
        ),
    );
    test.cargo_check()
        .unwrap_or_else(|e| panic!("range 0..n with n: {wj_ty} must compile: {e}"));
}

#[test]
fn range_start_matches_int_end() {
    check_width("int", "");
}

#[test]
fn range_start_matches_uint_end() {
    check_width("uint", "");
}

#[test]
fn range_start_matches_i32_end() {
    check_width("i32", "");
}

#[test]
fn range_start_matches_u32_end() {
    check_width("u32", "");
}

#[test]
fn range_counter_without_index_use_matches_int_end() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub fn count(n: int) -> int {
    let mut c: int = 0
    for i in 0..n {
        c = c + 1
    }
    c
}
"#,
    );
    test.cargo_check().unwrap_or_else(|e| panic!("{e}"));
}
