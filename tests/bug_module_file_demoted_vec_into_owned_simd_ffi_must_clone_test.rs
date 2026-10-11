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

//! demoted `&Vec<u32>` into owned FFI `Vec<u32>` must clone.
//!
//! Product residual (~32× Vec<u32>←&Vec), tip-out/gen graph_simd:
//!   wrapper `a: &Vec<u32>` forwards bare `a` into `*_ffi(a: Vec<u32>, …)`.
//! Twin of . Signature-driven.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str =
    include_str!("fixtures/library_multipass/demoted_vec_into_owned_ffi.wj");

#[test]
fn codegen_demoted_vec_into_owned_ffi_must_clone() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let demoted = rs.contains("fn count_intersection_readonly(a: &Vec<u32>")
        || rs.contains("count_intersection_readonly(a: &Vec");
    let bad = demoted
        && rs.contains("count_intersection_owned(a,")
        && !rs.contains("count_intersection_owned(a.clone(),");
    assert!(
        !bad,
        "demoted &Vec into owned FFI must clone. Generated:\n{rs}"
    );
    assert!(ok, "fixture must cargo-check. Generated:\n{rs}");
}
