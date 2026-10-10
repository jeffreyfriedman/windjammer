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

//! Regression: module file string lit into owned string must to string.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str =
    include_str!("fixtures/library_multipass/wdb221_string_lit_into_owned_string_must_to_string.wj");

#[test]
fn codegen_string_lit_into_owned_must_to_string() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let owned = rs.contains("fn needs_owned(kind: String") || rs.contains("needs_owned(kind: String");
    let bad = owned
        && rs.contains("needs_owned(\"Person_likes_Post\")")
        && !rs.contains("needs_owned(\"Person_likes_Post\".to_string())");
    eprintln!("codegen owned={} bad={}\n{rs}", owned, bad);
    assert!(
        !bad,
        "string lit into owned String must .to_string(). Generated:\n{rs}"
    );
    assert!(ok, "fixture must cargo-check. Generated:\n{rs}");
}

