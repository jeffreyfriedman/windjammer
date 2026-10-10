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

//! string lit / `.to_string()` into demoted `&str` columnar `sql_exec` must stay `&str`.
#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str = include_str!(
    "fixtures/library_multipass/string_lit_into_demoted_sql_exec_must_not_to_string.wj"
);

#[test]
fn codegen_string_lit_into_demoted_sql_exec_must_not_to_string() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let demoted = rs.contains("fn sql_exec(") && rs.contains("left_table: &str");
    let bad = rs.contains("\"props\".to_string()")
        || rs.contains("\"edges\".to_string()")
        || rs.contains("String::from(\"props\")");
    if demoted && bad {
        panic!(
            "RED demoted &str sql_exec received owned string lit. Generated:\n{rs}"
        );
    }
    assert!(ok, "fixture must cargo-check. Generated:\n{rs}");
}
