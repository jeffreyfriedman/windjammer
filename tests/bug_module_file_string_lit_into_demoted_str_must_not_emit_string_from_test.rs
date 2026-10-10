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

//! Regression: module file string lit into demoted str must not emit string from.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str = include_str!(
    "fixtures/library_multipass/wdb225_string_lit_into_demoted_str_must_not_emit_string_from.wj"
);

#[test]
fn codegen_string_lit_into_demoted_str_must_not_emit_string_from() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let demoted = rs.contains("fn join_path(root: &str")
        || rs.contains("name: &str")
        || (rs.contains("join_path") && rs.contains("&str"));
    let bad = rs.contains("String::from(\"BFS\")")
        && (rs.contains("join_path(root, String::from")
            || rs.contains("join_path(root,String::from"));
    if demoted && bad {
        panic!(
            "RED: demoted &str received String::from(\"BFS\"). Generated:\n{rs}"
        );
    }
    assert!(ok, "fixture must cargo-check. Generated:\n{rs}");
}

