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
    feature = "codegen_tests",
))]

//! Regression: module file owned string into demoted push cstring must borrow.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str =
    include_str!("fixtures/library_multipass/push_cstring_must_borrow.wj");

#[test]
fn codegen_push_cstring_must_borrow_owned_string_field() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let bad = rs.contains("push_cstring(f.name.clone()")
        || rs.contains("push_cstring(String::from(");
    assert!(
        !bad,
        "demoted &str push_cstring must borrow, not clone. Generated:\n{rs}"
    );
    assert!(ok, "fixture must cargo-check. Generated:\n{rs}");
}

