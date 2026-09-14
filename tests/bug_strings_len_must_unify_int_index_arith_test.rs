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

//! FAILING REPRO — `strings.len()` usize must unify with int loop indices.
//!
//! Product tip api-check (LedgerKit domain/string_contains.wj):
//!   `while i + n_len <= h_len` → WJ0003 expected int, found uint.

#[path = "common/test_utils.rs"]
mod test_utils;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/strings_len_must_unify_int_index_arith.wj");

#[test]
fn strings_len_must_unify_int_index_arith() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    if !ok {
        eprintln!("RED P3.267 strings.len int/uint:\n{rs}");
    }
    assert!(
        ok,
        "RED P3.267: strings.len() vs int index arith must cargo-check. Generated:\n{rs}"
    );
}
