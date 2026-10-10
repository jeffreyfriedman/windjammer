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

//! `u64` loop index must not compare against `len() as i64`.
#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str =
    include_str!("fixtures/library_multipass/wdb215_u64_index_len_compare.wj");

#[test]
fn codegen_u64_index_must_not_compare_len_as_i64() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let bad = rs.contains("len() as i64)") && rs.contains("while pi <");
    assert!(
        !bad,
        "u64 index vs len must not cast len to i64. Generated:\n{rs}"
    );
    assert!(ok, "fixture must cargo-check. Generated:\n{rs}");
}
